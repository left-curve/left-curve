use {
    crate::entities::{
        CandleInterval,
        candle::Candle,
        candle_query::{CandleQueryBuilder, MAX_ITEMS},
    },
    chrono::{DateTime, Utc},
    dango_types::dex::PairId,
    futures::future::join_all,
    std::collections::HashMap,
    strum::IntoEnumIterator,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CandleCacheKey {
    pub base_denom: String,
    pub quote_denom: String,
    pub interval: CandleInterval,
}

impl CandleCacheKey {
    pub fn new(base_denom: String, quote_denom: String, interval: CandleInterval) -> Self {
        Self {
            base_denom,
            quote_denom,
            interval,
        }
    }
}

#[derive(Debug, Default)]
pub struct CandleCache {
    candles: HashMap<CandleCacheKey, Vec<Candle>>,
}

impl CandleCache {
    pub fn add_candle(&mut self, key: CandleCacheKey, candle: Candle) {
        let candles = self.candles.entry(key).or_default();

        // Check if last candle has same time_start, if so replace it
        if let Some(last_candle) = candles.last_mut() {
            if last_candle.time_start == candle.time_start {
                *last_candle = candle;
                return;
            }
        }

        // Otherwise, add new candle
        candles.push(candle);
    }

    /// Does the cache have all candles for the given dates?
    pub fn date_interval_available(
        &self,
        key: &CandleCacheKey,
        earlier_than: Option<DateTime<Utc>>,
        later_than: Option<DateTime<Utc>>,
    ) -> bool {
        if let Some(candles) = self.candles.get(key) {
            if candles.is_empty() {
                return false;
            }

            if let Some(earlier_than) = earlier_than {
                if candles.last().is_some_and(|c| c.time_start > earlier_than) {
                    return false;
                }
            }

            if let Some(later_than) = later_than {
                if candles.first().is_some_and(|c| c.time_start < later_than) {
                    return false;
                }
            }

            return true;
        }

        false
    }

    pub fn get_candles(&self, key: &CandleCacheKey) -> Option<&Vec<Candle>> {
        self.candles.get(key)
    }

    pub fn get_last_candle(&self, key: &CandleCacheKey) -> Option<&Candle> {
        self.candles.get(key).and_then(|candles| candles.last())
    }

    /// Updates all existing pairs in the cache for a given block height.
    /// This will fetch the latest candles in parallel.
    pub async fn update_pairs(
        &mut self,
        clickhouse_client: &clickhouse::Client,
        pairs: &[PairId],
        block_height: u64,
    ) -> crate::error::Result<()> {
        // NOTE: Could potentially be optimized by using a single query to fetch all candles for
        // all pairs.

        let fetch_tasks = pairs
            .iter()
            .flat_map(|pair| {
                CandleInterval::iter().map(move |interval| {
                    let key = CandleCacheKey::new(
                        pair.base_denom.to_string(),
                        pair.quote_denom.to_string(),
                        interval,
                    );

                    async move {
                        let query_builder = CandleQueryBuilder::new(
                            key.interval,
                            key.base_denom.clone(),
                            key.quote_denom.clone(),
                        );

                        let candle = query_builder.fetch_one(clickhouse_client).await?;

                        Ok::<_, crate::error::IndexerError>((key, candle))
                    }
                })
            })
            .collect::<Vec<_>>();

        // Execute all fetches in parallel
        let results = join_all(fetch_tasks).await;

        // Process results
        for result in results {
            match result {
                Ok((key, None)) => {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        block_height,
                        base_denom = %key.base_denom,
                        quote_denom = %key.quote_denom,
                        interval = %key.interval,
                        "No candle found",
                    );
                },
                Ok((key, Some(fetched_candle))) => {
                    self.add_candle(key.clone(), fetched_candle.clone());
                },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(err = %_err, "Failed to preload candles");
                },
            }
        }

        Ok(())
    }

    /// Preloads candles in parallel.
    pub async fn preload_pairs(
        &mut self,
        pairs: &[PairId],
        clickhouse_client: &clickhouse::Client,
    ) -> crate::error::Result<()> {
        // Create all fetch tasks
        let fetch_tasks = pairs
            .iter()
            .flat_map(|pair| {
                CandleInterval::iter().map(move |interval| {
                    let key = CandleCacheKey::new(
                        pair.base_denom.to_string(),
                        pair.quote_denom.to_string(),
                        interval,
                    );

                    async move {
                        let query_builder = CandleQueryBuilder::new(
                            key.interval,
                            key.base_denom.clone(),
                            key.quote_denom.clone(),
                        )
                        .with_limit(MAX_ITEMS);

                        let mut candles = query_builder.fetch_all(clickhouse_client).await?.candles;
                        candles.reverse(); // Most recent first -> most recent last

                        Ok::<_, crate::error::IndexerError>((key, candles))
                    }
                })
            })
            .collect::<Vec<_>>();

        // Execute all fetches in parallel
        let results = join_all(fetch_tasks).await;

        // Process results
        for result in results {
            match result {
                Ok((key, candles)) => {
                    *self.candles.entry(key).or_default() = candles;
                },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(err = %_err, "Failed to preload candles");
                },
            }
        }

        Ok(())
    }

    // Keep last N candles
    pub fn compact_keep_n(&mut self, n: usize) {
        self.candles.retain(|_key, candles| {
            if candles.is_empty() {
                false
            } else {
                // Keep only last N candles
                let start = candles.len().saturating_sub(n);
                candles.drain(..start);
                true
            }
        });
    }
}
