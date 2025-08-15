#[cfg(feature = "metrics")]
use metrics::{counter, gauge, histogram};

use {
    crate::{
        entities::{
            CandleInterval,
            candle::Candle,
            candle_query::{CandleQueryBuilder, MAX_ITEMS},
            pair_price::PairPrice,
        },
        error::{IndexerError, Result},
    },
    chrono::{DateTime, Utc},
    dango_types::dex::PairId,
    futures::future::join_all,
    itertools::Itertools,
    std::{
        collections::{HashMap, HashSet},
        time::{Duration, Instant},
    },
    strum::IntoEnumIterator,
    tokio::time::sleep,
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
            #[cfg(feature = "tracing")]
            tracing::debug!(block_height, "Received empty pair_prices");
            return;
        }

        #[cfg(feature = "tracing")]
        tracing::debug!(block_height, ?pair_prices, "Adding pair_prices");

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

                // #[cfg(feature = "tracing")]
                // tracing::debug!(
                //     %candle.block_height,
                //     %candle.base_denom,
                //     %candle.quote_denom,
                //     %candle.volume_base,
                //     %candle.volume_quote,
                //     %candle_interval,
                //     "Calling add_candle()",
                // );

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
        if candles.is_empty() {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                %candle.block_height,
                %candle.base_denom,
                %candle.quote_denom,
                "Adding candle, cache is empty",
            );

            candles.push(candle);
            return;
        };

        // NOTE: Candles don't necessarily come in order, because the indexing
        // is done async per block. We could receive block 5 before block 4.
        let existing_candle = candles
            .iter_mut()
            .rev()
            .take_while(|existing_candle| existing_candle.time_start >= candle.time_start)
            .find(|existing_candle| existing_candle.time_start == candle.time_start);

        let Some(existing_candle) = existing_candle else {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                %candle.block_height,
                %candle.base_denom,
                %candle.quote_denom,
                "Pushing candle, no existing candle found",
            );

            // Find correct position to maintain time order
            let insert_pos = candles
                .iter()
                .position(|c| c.time_start > candle.time_start)
                .unwrap_or(candles.len());

            candles.insert(insert_pos, candle);
            return;
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            %candle.block_height,
            %candle.base_denom,
            %candle.quote_denom,
            %existing_candle.volume_base,
            %existing_candle.volume_quote,
            %existing_candle.block_height,
            %candle.volume_base,
            %candle.volume_quote,
            "Modifying existing candle",
        );

        if candle.block_height > existing_candle.block_height {
            candle.open = existing_candle.open;
        } else {
            candle.close = existing_candle.close;
        }

        candle.high = existing_candle.high.max(candle.high);
        candle.low = existing_candle.low.min(candle.low);

        candle.volume_base += existing_candle.volume_base;
        candle.volume_quote += existing_candle.volume_quote;

        candle.block_height = existing_candle.block_height.max(candle.block_height);

        *existing_candle = candle;
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
                                        %key.base_denom,
                                        %key.quote_denom,
                                        %key.interval,
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

#[cfg(test)]
mod tests {
    use {
        super::*,
        assertor::*,
        chrono::NaiveDateTime,
        grug::{Denom, Udec128_6, Udec128_24},
        std::{collections::VecDeque, str::FromStr},
    };

    #[tokio::test]
    async fn create_candles() -> Result<()> {
        let mut candle_cache = CandleCache::default();

        for pair_price in parsed_pair_prices()? {
            let block_height = pair_price.block_height;

            let hashmap_pair_price = HashMap::from([(
                PairId {
                    base_denom: Denom::from_str(&pair_price.base_denom)?,
                    quote_denom: Denom::from_str(&pair_price.quote_denom)?,
                },
                pair_price,
            )]);

            candle_cache.add_pair_prices(block_height, hashmap_pair_price);
        }

        let cache_key = CandleCacheKey::new(
            "bridge/btc".to_string(),
            "bridge/usdc".to_string(),
            CandleInterval::OneSecond,
        );

        let mut candles: VecDeque<Candle> = candle_cache
            .get_candles(&cache_key)
            .expect("No candles found")
            .clone()
            .into();

        let mut previous_candle = candles.pop_front().expect("No previous candle found");

        while let Some(candle) = candles.pop_front() {
            assert!(
                candle.time_start > previous_candle.time_start,
                "Candle time_start is not greater than previous candle"
            );
            assert!(
                candle.block_height >= previous_candle.block_height,
                "Candle block_height is not greater than or equal to previous candle"
            );
            assert_eq!(
                previous_candle.close, candle.open,
                "Candle close price does not match next candle open price"
            );

            previous_candle = candle;
        }

        Ok(())
    }

    // Ensure when candles aren't coming in order, they're grouped properly
    #[tokio::test]
    async fn when_pair_prices_are_not_in_order() -> Result<()> {
        let mut candle_cache = CandleCache::default();

        let quote_denom = "bridge/usdc";
        let base_denom = "bridge/btc";

        let pair_prices = vec![
            pair_price(
                quote_denom,
                base_denom,
                1217208030172232059779705322,
                1217208030172232059779705322,
                1217208030172232059779705322,
                1217208030172232059779705322,
                0,
                0,
                "2025-08-13 17:36:01.038565",
                4,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                1217208030172232059779705322,
                1217208030172232059779705322,
                1217208030172232059779705322,
                1217208030172232059779705322,
                0,
                0,
                "2025-08-13 17:36:00.086616",
                2,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                1217208030172232059779705322,
                1216963512618701957116501833,
                1216963512618701957116501833,
                1216963512618701957116501833,
                345160391948092,
                420047603001999188,
                "2025-08-13 17:36:01.734583",
                6,
            )?,
        ];

        for pair_price in pair_prices {
            let block_height = pair_price.block_height;

            let hashmap_pair_price = HashMap::from([(
                PairId {
                    base_denom: Denom::from_str(&pair_price.base_denom)?,
                    quote_denom: Denom::from_str(&pair_price.quote_denom)?,
                },
                pair_price,
            )]);

            candle_cache.add_pair_prices(block_height, hashmap_pair_price);
        }

        let cache_key = CandleCacheKey::new(
            base_denom.to_string(),
            quote_denom.to_string(),
            CandleInterval::OneSecond,
        );

        assert_that!(candle_cache.get_candles(&cache_key).cloned().unwrap()).has_length(2);

        Ok(())
    }

    fn parsed_pair_prices() -> Result<Vec<PairPrice>> {
        let quote_denom = "bridge/usdc";
        let base_denom = "bridge/btc";

        Ok(vec![
            pair_price(
                quote_denom,
                base_denom,
                1217208030172232059779705322,
                1217208030172232059779705322,
                1217208030172232059779705322,
                1217208030172232059779705322,
                0,
                0,
                "2025-08-13 17:36:00.038565",
                1277884,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                1217208030172232059779705322,
                1217208030172232059779705322,
                1217208030172232059779705322,
                1217208030172232059779705322,
                0,
                0,
                "2025-08-13 17:36:01.086616",
                1277885,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                1217208030172232059779705322,
                1216963512618701957116501833,
                1216963512618701957116501833,
                1216963512618701957116501833,
                345160391948092,
                420047603001999188,
                "2025-08-13 17:36:02.134583",
                1277886,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                1216963512618701957116501833,
                1216961816562295438187182730,
                1216961816562295438187182730,
                1216961816562295438187182730,
                108404821869154,
                131924528945998920,
                "2025-08-13 17:36:03.182534",
                1277887,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                1216961816562295438187182730,
                1216961816562295438187182730,
                1216961816562295438187182730,
                1216961816562295438187182730,
                0,
                0,
                "2025-08-13 17:36:04.230527",
                1277888,
            )?,
        ])
    }

    fn pair_price(
        quote_denom: &str,
        base_denom: &str,
        open_price: u128,
        highest_price: u128,
        lowest_price: u128,
        close_price: u128,
        volume_base: u128,
        volume_quote: u128,
        created_at: &str,
        block_height: u64,
    ) -> Result<PairPrice> {
        Ok(PairPrice {
            quote_denom: quote_denom.to_string(),
            base_denom: base_denom.to_string(),
            open_price: Udec128_24::raw(grug::Int::new(open_price)),
            highest_price: Udec128_24::raw(grug::Int::new(highest_price)),
            lowest_price: Udec128_24::raw(grug::Int::new(lowest_price)),
            close_price: Udec128_24::raw(grug::Int::new(close_price)),
            volume_base: Udec128_6::raw(grug::Int::new(volume_base)),
            volume_quote: Udec128_6::raw(grug::Int::new(volume_quote)),
            created_at: NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S%.f")?
                .and_utc(),
            block_height,
        })
    }
}
