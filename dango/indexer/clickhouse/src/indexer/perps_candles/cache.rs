#[cfg(feature = "metrics")]
use metrics::{counter, gauge, histogram};
use {
    crate::entities::{
        CandleInterval,
        perps_candle::PerpsCandle,
        perps_candle_query::{MAX_ITEMS, PerpsCandleQueryBuilder},
        perps_pair_price::PerpsPairPrice,
    },
    chrono::{DateTime, Utc},
    futures::future::join_all,
    itertools::Itertools,
    std::{
        collections::{HashMap, HashSet},
        time::Instant,
    },
    strum::IntoEnumIterator,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct PerpsCandleCacheKey {
    pub pair_id: String,
    pub interval: CandleInterval,
}

impl PerpsCandleCacheKey {
    pub fn new(pair_id: String, interval: CandleInterval) -> Self {
        Self { pair_id, interval }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct PerpsCandleCache {
    pub candles: HashMap<PerpsCandleCacheKey, Vec<PerpsCandle>>,
    pub pair_prices: HashMap<u64, Vec<PerpsPairPrice>>,
    pub completed_candles: HashSet<PerpsCandle>,
    pair_ids: HashSet<String>,
}

impl PerpsCandleCache {
    pub fn pair_price_for_block(&self, block_height: u64) -> Option<&Vec<PerpsPairPrice>> {
        self.pair_prices.get(&block_height)
    }

    /// Returns true if there are gaps in the cached pair prices
    pub fn has_gaps(&self) -> bool {
        let mut heights: Vec<u64> = self.pair_prices.keys().copied().collect();
        heights.sort_unstable();

        for window in heights.windows(2) {
            if let [a, b] = window
                && *b != *a + 1
            {
                return true;
            }
        }

        false
    }

    pub fn add_pair_prices(
        &mut self,
        block_height: u64,
        created_at: DateTime<Utc>,
        pair_prices: Vec<PerpsPairPrice>,
    ) -> Vec<PerpsCandle> {
        #[cfg(feature = "tracing")]
        tracing::debug!(
            block_height,
            pair_prices_count = pair_prices.len(),
            "Adding perps pair_prices",
        );

        let mut result = Vec::new();

        let mut seen_pair_ids: HashSet<String> = HashSet::new();

        for pair_price in pair_prices.iter() {
            for candle_interval in CandleInterval::iter() {
                seen_pair_ids.insert(pair_price.pair_id.clone());

                let key = PerpsCandleCacheKey::new(pair_price.pair_id.clone(), candle_interval);

                if let Some(candle) = self.update_or_create_candle(
                    key,
                    block_height,
                    created_at,
                    Some(pair_price.clone()),
                ) {
                    result.push(candle);
                }
            }
        }

        // Keep creating candles for pairs without new prices in this block
        for pair_id in self.pair_ids.clone() {
            if seen_pair_ids.contains(&pair_id) {
                continue;
            }

            for candle_interval in CandleInterval::iter() {
                let key = PerpsCandleCacheKey::new(pair_id.clone(), candle_interval);

                if let Some(candle) =
                    self.update_or_create_candle(key, block_height, created_at, None)
                {
                    result.push(candle);
                }
            }
        }

        self.pair_ids.extend(seen_pair_ids);

        self.pair_prices
            .entry(block_height)
            .or_default()
            .extend(pair_prices);

        result
    }

    pub fn update_or_create_candle(
        &mut self,
        key: PerpsCandleCacheKey,
        block_height: u64,
        created_at: DateTime<Utc>,
        pair_price: Option<PerpsPairPrice>,
    ) -> Option<PerpsCandle> {
        let time_start = key.interval.interval_start(created_at);
        let interval = key.interval;
        let candles = self.candles.entry(key.clone()).or_default();

        // No previous existing candles, we can just push a new candle
        if candles.is_empty() {
            #[allow(clippy::question_mark)]
            let Some(pair_price) = pair_price else {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    %created_at,
                    %interval,
                    "Perps candles is empty, no pair_price, can not create candle",
                );

                return None;
            };

            let candle =
                PerpsCandle::new_with_pair_price(&pair_price, interval, time_start, block_height);

            candles.push(candle.clone());

            return Some(candle);
        }

        let current_index = candles
            .iter()
            .rposition(|candle| candle.time_start == time_start);

        let Some(current_index) = current_index else {
            // Find correct position to maintain time order
            let insert_pos = candles
                .iter()
                .position(|c| c.time_start > time_start)
                .unwrap_or(candles.len());

            let Some(pair_price) = pair_price else {
                if let Some(previous_candle) = insert_pos
                    .checked_sub(1)
                    .and_then(|idx| candles.get(idx))
                    .cloned()
                {
                    let candle = PerpsCandle::new_with_previous_candle(
                        &previous_candle,
                        interval,
                        time_start,
                        block_height,
                    );

                    candles.insert(insert_pos, candle.clone());

                    self.completed_candles.replace(previous_candle);

                    return Some(candle);
                } else {
                    return None;
                }
            };

            let mut candle =
                PerpsCandle::new_with_pair_price(&pair_price, interval, time_start, block_height);

            // Set open price from previous candle close
            if let Some(previous_candle) = insert_pos
                .checked_sub(1)
                .and_then(|idx| candles.get(idx))
                .cloned()
            {
                candle.open = previous_candle.close;
                candle.set_high_low(previous_candle.close, previous_candle.close);

                self.completed_candles.replace(previous_candle);
            }

            candles.insert(insert_pos, candle.clone());

            // Update the open price of the next candle, if exists.
            if let Some(next_candle) = candles.get_mut(insert_pos + 1) {
                next_candle.open = pair_price.close;
                next_candle.set_high_low(pair_price.close, pair_price.close);
            }

            return Some(candle);
        };

        if block_height == candles[current_index].max_block_height {
            #[cfg(feature = "tracing")]
            tracing::error!(
                %interval,
                %block_height,
                "Seeing the same perps pair_price for the same block_height",
            );
        }

        if let Some(pair_price) = pair_price {
            candles[current_index].volume += pair_price.volume;
            candles[current_index].volume_usd += pair_price.volume_usd;

            candles[current_index].set_high_low(pair_price.high, pair_price.low);

            if block_height < candles[current_index].min_block_height {
                if current_index.checked_sub(1).is_none() {
                    candles[current_index].open = pair_price.close;
                }

                candles[current_index].set_high_low(pair_price.high, pair_price.low);
            }

            // Set close price from latest block
            if block_height > candles[current_index].max_block_height {
                candles[current_index].close = pair_price.close;
                candles[current_index].set_high_low(pair_price.high, pair_price.low);

                if let Some(next_candle) = candles.get_mut(current_index + 1) {
                    next_candle.open = pair_price.close;
                    next_candle.set_high_low(pair_price.close, pair_price.close);
                }
            }
        }

        if block_height < candles[current_index].min_block_height {
            candles[current_index].min_block_height = block_height;
        }

        if block_height > candles[current_index].max_block_height {
            candles[current_index].max_block_height = block_height;
        }

        Some(candles[current_index].clone())
    }

    /// Does the cache have all candles for the given dates?
    pub fn date_interval_available(
        &self,
        key: &PerpsCandleCacheKey,
        earlier_than: Option<DateTime<Utc>>,
        later_than: Option<DateTime<Utc>>,
    ) -> bool {
        if let Some(candles) = self.candles.get(key) {
            if candles.is_empty() {
                return false;
            }

            if let Some(earlier_than) = earlier_than
                && candles.last().is_some_and(|c| c.time_start > earlier_than)
            {
                return false;
            }

            if let Some(later_than) = later_than
                && candles.first().is_some_and(|c| c.time_start < later_than)
            {
                return false;
            }

            return true;
        }

        false
    }

    pub fn get_candles(&self, key: &PerpsCandleCacheKey) -> Option<&Vec<PerpsCandle>> {
        self.candles.get(key)
    }

    pub fn get_last_candle(&self, key: &PerpsCandleCacheKey) -> Option<&PerpsCandle> {
        let _start = Instant::now();

        let result = self.candles.get(key).and_then(|candles| candles.last());

        #[cfg(feature = "metrics")]
        {
            if result.is_some() {
                counter!("indexer.clickhouse.perps_candles.cache.hits").increment(1);
            } else {
                counter!("indexer.clickhouse.perps_candles.cache.misses").increment(1);
            }

            let duration = _start.elapsed();
            histogram!("indexer.clickhouse.perps_candles.cache.lookup.duration.seconds")
                .record(duration.as_secs_f64());
        }

        result
    }

    /// Preloads candles in parallel.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    pub async fn preload_pairs(
        &mut self,
        pair_ids: &[String],
        clickhouse_client: &clickhouse::Client,
    ) -> crate::error::Result<()> {
        let last_prices = PerpsPairPrice::latest_prices(clickhouse_client, MAX_ITEMS).await?;

        let Some(highest_block_height) = last_prices.last().map(|price| price.block_height) else {
            #[cfg(feature = "tracing")]
            tracing::warn!(
                "No perps last prices found, skipping perps candle preload since it's all empty."
            );
            return Ok(());
        };

        self.pair_prices.extend(last_prices.into_iter().fold(
            HashMap::new(),
            |mut acc: HashMap<u64, Vec<PerpsPairPrice>>, price| {
                acc.entry(price.block_height).or_default().push(price);
                acc
            },
        ));

        // Add all pair_ids
        let all_pair_ids = PerpsPairPrice::all_pair_ids(clickhouse_client).await?;
        self.pair_ids.extend(all_pair_ids);

        // Create all fetch tasks
        let fetch_tasks = pair_ids
            .iter()
            .flat_map(|pair_id| {
                CandleInterval::iter().map(move |interval| {
                    let key = PerpsCandleCacheKey::new(pair_id.clone(), interval);

                    async move {
                        let query_builder =
                            PerpsCandleQueryBuilder::new(key.interval, key.pair_id.clone())
                                .with_limit(MAX_ITEMS);

                        let mut candles = query_builder.fetch_all(clickhouse_client).await?.candles;

                        if let Some(candle) = candles.first()
                            && candle.max_block_height < highest_block_height
                        {
                            #[cfg(feature = "tracing")]
                            tracing::info!(
                                %candle.max_block_height,
                                %highest_block_height,
                                %key.pair_id,
                                %key.interval,
                                "Perps candle is older than latest price."
                            );
                        }

                        candles.reverse();

                        Ok::<_, crate::error::IndexerError>((key, candles))
                    }
                })
            })
            .collect::<Vec<_>>();

        let results = join_all(fetch_tasks).await;

        for result in results {
            match result {
                Ok((key, candles)) => {
                    *self.candles.entry(key).or_default() = candles;
                },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!(err = %_err, "Failed to preload perps candles");
                },
            }
        }

        #[cfg(feature = "tracing")]
        tracing::info!("Preloaded all perps candles");

        self.update_metrics();

        Ok(())
    }

    // Keep last N candles, store the rest on Clickhouse
    pub fn compact_keep_n(&mut self, n: usize) {
        self.candles.retain(|_key, candles| {
            if candles.is_empty() {
                false
            } else {
                let start = candles.len().saturating_sub(n);
                let _drained: Vec<_> = candles.drain(..start).collect();

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
            gauge!("indexer.clickhouse.perps_candles.cache.size.entries")
                .set(self.candles.len() as f64);

            let total_candles: usize = self.candles.values().map(Vec::len).sum();
            gauge!("indexer.clickhouse.perps_candles.cache.size.candles").set(total_candles as f64);

            gauge!("indexer.clickhouse.perps_pair_prices.cache.size.entries")
                .set(self.pair_prices.len() as f64);

            let total_pair_prices: usize = self.pair_prices.values().map(Vec::len).sum();
            gauge!("indexer.clickhouse.perps_pair_prices.cache.size.pair_prices")
                .set(total_pair_prices as f64);
        }
    }
}
