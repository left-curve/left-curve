use std::time::Duration;

#[cfg(feature = "metrics")]
use metrics::{counter, gauge, histogram};
use tokio::time::sleep;

use crate::error::IndexerError;

use {
    crate::{
        entities::{
            CandleInterval,
            candle::Candle,
            candle_query::{CandleQueryBuilder, MAX_ITEMS},
            pair_price::PairPrice,
        },
        error::Result,
    },
    chrono::{DateTime, Utc},
    dango_types::dex::PairId,
    futures::future::join_all,
    itertools::Itertools,
    std::{
        collections::{HashMap, HashSet},
        time::Instant,
    },
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

#[derive(Debug, Default, Eq, PartialEq)]
pub struct CandleCache {
    pub candles: HashMap<CandleCacheKey, Vec<Candle>>,
    pub pair_prices: HashMap<u64, HashMap<PairId, PairPrice>>,
}

impl CandleCache {
    pub fn pair_price_for_block(&self, block_height: u64) -> Option<&HashMap<PairId, PairPrice>> {
        self.pair_prices.get(&block_height)
    }

    pub fn add_pair_prices(&mut self, block_height: u64, pair_prices: HashMap<PairId, PairPrice>) {
        if pair_prices.is_empty() {
            return;
        }

        for pair_price in pair_prices.values() {
            for candle_interval in CandleInterval::iter() {
                let key = CandleCacheKey::new(
                    pair_price.base_denom.clone(),
                    pair_price.quote_denom.clone(),
                    candle_interval,
                );

                let candle = Candle {
                    quote_denom: pair_price.quote_denom.clone(),
                    base_denom: pair_price.base_denom.clone(),
                    time_start: candle_interval.interval_start(pair_price.created_at),
                    open: pair_price.open_price,
                    high: pair_price.highest_price,
                    low: pair_price.lowest_price,
                    close: pair_price.close_price,
                    volume_base: pair_price.volume_base,
                    volume_quote: pair_price.volume_quote,
                    interval: candle_interval,
                    block_height: pair_price.block_height,
                };

                self.add_candle(key, candle);
            }
        }

        self.pair_prices
            .entry(block_height)
            .or_default()
            .extend(pair_prices);
    }

    pub fn add_candle(&mut self, key: CandleCacheKey, mut candle: Candle) {
        let candles = self.candles.entry(key).or_default();

        // no existing candles, we can just push it
        let Some(last_candle) = candles.last_mut() else {
            candles.push(candle);
            return;
        };

        // received candle is older, we can ignore it
        if last_candle.block_height >= candle.block_height {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                block_height = candle.block_height,
                last_block_height = last_candle.block_height,
                base_denom = candle.base_denom,
                quote_denom = candle.quote_denom,
                "Ignoring candle",
            );

            return;
        }

        // Check if last candle has same time_start, if so replace it and update
        // max/min/open/close values. Candles are coming in order.
        if last_candle.time_start == candle.time_start {
            // Keep the original open price from the interval start
            candle.open = last_candle.open;
            candle.high = last_candle.high.max(candle.high);
            candle.low = last_candle.low.min(candle.low);
            candle.volume_base += last_candle.volume_base;
            candle.volume_quote += last_candle.volume_quote;

            *last_candle = candle;
        } else {
            candles.push(candle);
        }
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
        let _start = Instant::now();

        let result = self.candles.get(key).and_then(|candles| candles.last());

        #[cfg(feature = "metrics")]
        {
            if result.is_some() {
                counter!("indexer.clickhouse.candles.cache.hits").increment(1);
            } else {
                counter!("indexer.clickhouse.candles.cache.misses").increment(1);
            }

            let duration = _start.elapsed();
            histogram!("indexer.clickhouse.candles.cache.lookup.duration.seconds")
                .record(duration.as_secs_f64());
        }

        result
    }

    /// Preloads candles in parallel.
    pub async fn preload_pairs(
        &mut self,
        pairs: &[PairId],
        clickhouse_client: &clickhouse::Client,
    ) -> Result<()> {
        let last_prices = PairPrice::latest_prices(clickhouse_client, MAX_ITEMS)
            .await?
            .into_iter()
            .map(|price| Ok(((&price).try_into()?, price)))
            .filter_map(Result::ok)
            .fold(
                HashMap::<u64, HashMap<PairId, PairPrice>>::new(),
                |mut acc, (pair_id, price)| {
                    acc.entry(price.block_height)
                        .or_default()
                        .insert(pair_id, price);
                    acc
                },
            );

        let Some(highest_block_height) = last_prices.keys().copied().max() else {
            #[cfg(feature = "tracing")]
            tracing::warn!("No last prices found, skipping candle preload since it's all empty.");
            return Ok(());
        };

        self.pair_prices.extend(last_prices);

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
                        let mut candles;
                        let start = Instant::now();

                        loop {
                            if start.elapsed() > Duration::from_secs(2) {
                                #[cfg(feature = "tracing")]
                                tracing::warn!(
                                    "Timeout while preloading candles for {}-{}",
                                    key.base_denom,
                                    key.quote_denom
                                );
                                return Err(IndexerError::CandleTimeout);
                            }

                            let query_builder = CandleQueryBuilder::new(
                                key.interval,
                                key.base_denom.clone(),
                                key.quote_denom.clone(),
                            )
                            .with_limit(MAX_ITEMS);

                            candles = query_builder.fetch_all(clickhouse_client).await?.candles;

                            if let Some(candle) = candles.first() {
                                if candle.block_height < highest_block_height {
                                    #[cfg(feature = "tracing")]
                                    tracing::warn!(
                                        %candle.block_height,
                                        %highest_block_height,
                                        "Candle is older than latest price");

                                    // `candle` are built async in clickhouse, and this means they're
                                    // not synced to the latest block yet.
                                    // This won't happen in production, `preload_pairs` is called at start
                                    // but during tests, it can happen.
                                    sleep(Duration::from_millis(100)).await;

                                    continue;
                                }
                            }

                            candles.reverse(); // Most recent first -> most recent last

                            break;
                        }

                        Ok::<_, IndexerError>((key, candles))
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

        self.update_metrics();

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

        let top_n_keys: HashSet<u64> = self
            .pair_prices
            .keys()
            .copied()
            .sorted_by(|a, b| b.cmp(a))
            .take(n)
            .collect();

        self.pair_prices.retain(|k, _| top_n_keys.contains(k));
    }

    fn update_metrics(&self) {
        #[cfg(feature = "metrics")]
        {
            // Number of unique cache keys (trading pairs × intervals)
            gauge!("indexer.clickhouse.candles.cache.size.entries").set(self.candles.len() as f64);

            // Total individual candles stored
            let total_candles: usize = self.candles.values().map(Vec::len).sum();
            gauge!("indexer.clickhouse.candles.cache.size.candles").set(total_candles as f64);

            // Number of unique cache keys
            gauge!("indexer.clickhouse.pair_prices.cache.size.entries")
                .set(self.pair_prices.len() as f64);

            // Total individual pair_prices stored
            let total_pair_prices: usize = self.pair_prices.values().map(HashMap::len).sum();
            gauge!("indexer.clickhouse.pair_prices.cache.size.pair_prices")
                .set(total_pair_prices as f64);
        }
    }
}
