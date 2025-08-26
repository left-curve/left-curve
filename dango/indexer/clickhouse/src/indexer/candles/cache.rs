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
    pub pair_prices: HashMap<u64, Vec<PairPrice>>,
    // All denoms we've seen so far
    denoms: HashSet<PairId>,
}

impl CandleCache {
    pub fn pair_price_for_block(&self, block_height: u64) -> Option<&Vec<PairPrice>> {
        self.pair_prices.get(&block_height)
    }

    #[allow(dead_code)]
    /// Returns true if there are gaps in the cached pair prices
    fn has_gaps(self) -> bool {
        let mut heights: Vec<u64> = self.pair_prices.keys().copied().collect();
        heights.sort_unstable();

        for window in heights.windows(2) {
            if let [a, b] = window {
                if *b != *a + 1 {
                    return true;
                }
            }
        }

        false
    }

    pub fn add_pair_prices(
        &mut self,
        block_height: u64,
        created_at: DateTime<Utc>,
        pair_prices: Vec<PairPrice>,
    ) -> Vec<Candle> {
        #[cfg(feature = "tracing")]
        tracing::debug!(block_height, ?pair_prices, "Adding pair_prices");

        let mut result = Vec::new();

        let mut seen_pair_prices: HashSet<PairId> = HashSet::new();

        for pair_price in pair_prices.iter() {
            for candle_interval in CandleInterval::iter() {
                if let Ok(denom) = pair_price.try_into() {
                    // self.denoms.insert(denom);
                    seen_pair_prices.insert(denom);
                }

                let key = CandleCacheKey::new(
                    pair_price.base_denom.clone(),
                    pair_price.quote_denom.clone(),
                    candle_interval,
                );

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

        // This is so we keep creating candles even if we don't have new pair prices in this block
        for pair_price in self.denoms.clone() {
            if seen_pair_prices.contains(&pair_price) {
                continue;
            }

            for candle_interval in CandleInterval::iter() {
                let key = CandleCacheKey::new(
                    pair_price.base_denom.to_string(),
                    pair_price.quote_denom.to_string(),
                    candle_interval,
                );

                if let Some(candle) = self.update_or_create_candle(
                    key,
                    block_height,
                    created_at,
                    None, // No new pair_price
                ) {
                    result.push(candle);
                }
            }
        }

        self.denoms.extend(seen_pair_prices);

        self.pair_prices
            .entry(block_height)
            .or_default()
            .extend(pair_prices);

        result
    }

    /// This is also called when we don't have new pair_prices but need to create/update a candle
    pub fn update_or_create_candle(
        &mut self,
        key: CandleCacheKey,
        block_height: u64,
        created_at: DateTime<Utc>,
        pair_price: Option<PairPrice>,
    ) -> Option<Candle> {
        let time_start = key.interval.interval_start(created_at);
        let interval = key.interval;
        let candles = self.candles.entry(key.clone()).or_default();

        // no previous existing candles, we can just push a new candle
        if candles.is_empty() {
            #[allow(clippy::question_mark)]
            let Some(pair_price) = pair_price else {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    %created_at,
                    %interval,
                    "Candles is empty, no pair_price, creating no candle",
                );

                return None;
            };

            #[cfg(feature = "tracing")]
            tracing::debug!(
                %pair_price.block_height,
                %pair_price.base_denom,
                %pair_price.quote_denom,
                %interval,
                "Candles is empty, adding a new candle",
            );

            let candle =
                Candle::new_with_pair_price(pair_price, interval, time_start, block_height);

            candles.push(candle.clone());

            return Some(candle);
        }

        // NOTE: Candles don't necessarily come in order, because the indexing
        // is done async per block. We could receive block 5 before block 4.
        // The existing candle could be an older candle than our last.
        let existing_candle = candles
            .iter_mut()
            .rev()
            .take_while(|existing_candle| existing_candle.time_start >= time_start)
            .find(|existing_candle| existing_candle.time_start == time_start);

        let Some(existing_candle) = existing_candle else {
            // Find correct position to maintain time order
            let insert_pos = candles
                .iter()
                .position(|c| c.time_start > time_start)
                .unwrap_or(candles.len());

            let Some(pair_price) = pair_price else {
                if insert_pos > 0 {
                    let previous_candle = &candles[insert_pos - 1];

                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        %created_at,
                        %interval,
                        "Found no existing candle, and no pair_price, will create a candle based on previous candle",
                    );

                    let candle = Candle::new_with_previous_candle(
                        previous_candle,
                        interval,
                        time_start,
                        block_height,
                    );

                    candles.insert(insert_pos, candle.clone());

                    return Some(candle);
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        %created_at,
                        %interval,
                        "Found no existing candle, no pair_price, no previous candle, creating no candle",
                    );
                    return None;
                }
            };

            #[cfg(feature = "tracing")]
            tracing::debug!(
                %block_height,
                %key.base_denom,
                %key.quote_denom,
                %interval,
                "No existing candle found, adding a new candle",
            );

            let mut candle =
                Candle::new_with_pair_price(pair_price, interval, time_start, block_height);

            // We set the open price for the new candle to the previous candle close price
            if insert_pos > 0 {
                let previous_candle = &candles[insert_pos - 1];
                candle.open = previous_candle.close;
            }

            candles.insert(insert_pos, candle.clone());

            return Some(candle);
        };

        #[cfg(feature = "tracing")]
        tracing::debug!(
            %block_height,
            %key.base_denom,
            %key.quote_denom,
            %interval,
            %existing_candle.volume_base,
            %existing_candle.volume_quote,
            %existing_candle.block_height,
            "Modifying existing candle",
        );

        if block_height == existing_candle.block_height {
            #[cfg(feature = "tracing")]
            tracing::error!(
                %interval,
                %block_height,
                "Seeing the same pair_price for the same block_height",
            );
        } else if block_height > existing_candle.block_height {
            // PairPrice might not come in order, we only set close price if the
            // pair price has a later block.
            // NOTE: we should also change the open price of the next candle, if exists.
            if let Some(pair_price) = &pair_price {
                existing_candle.close = pair_price.clearing_price;
            }
        }

        if let Some(pair_price) = pair_price {
            existing_candle.volume_base += pair_price.volume_base;
            existing_candle.volume_quote += pair_price.volume_quote;

            existing_candle.high = existing_candle.high.max(pair_price.clearing_price);
            existing_candle.low = existing_candle.low.min(pair_price.clearing_price);
        }

        existing_candle.block_height = existing_candle.block_height.max(block_height);
        existing_candle.blocks_count += 1;

        Some(existing_candle.clone())
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
        let last_prices = PairPrice::latest_prices(clickhouse_client, MAX_ITEMS).await?;

        let Some(highest_block_height) = last_prices.last().map(|price| price.block_height) else {
            #[cfg(feature = "tracing")]
            tracing::warn!("No last prices found, skipping candle preload since it's all empty.");
            return Ok(());
        };

        self.pair_prices.extend(last_prices.into_iter().fold(
            HashMap::new(),
            |mut acc: HashMap<u64, Vec<PairPrice>>, price| {
                acc.entry(price.block_height).or_default().push(price);
                acc
            },
        ));

        // Add all denoms
        let all_pairs = PairPrice::all_pairs(clickhouse_client).await?;
        self.denoms.extend(all_pairs);

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

    pub fn store_candles_in_clickhouse(_candles: Vec<Candle>) {
        todo!()
    }

    // Keep last N candles, store the rest on Clickhouse
    pub fn compact_keep_n(&mut self, n: usize) {
        self.candles.retain(|_key, candles| {
            if candles.is_empty() {
                false
            } else {
                // Keep only last N candles
                let start = candles.len().saturating_sub(n);
                let drained: Vec<_> = candles.drain(..start).collect();

                // Store drained candles in Clickhouse
                Self::store_candles_in_clickhouse(drained);

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
            let total_pair_prices: usize = self.pair_prices.values().map(Vec::len).sum();
            gauge!("indexer.clickhouse.pair_prices.cache.size.pair_prices")
                .set(total_pair_prices as f64);
        }
    }
}

#[cfg(test)]
impl CandleCache {
    pub fn add_multi_block_pair_prices(&mut self, pair_prices: Vec<PairPrice>) -> Result<()> {
        for pair_price in pair_prices {
            let block_height = pair_price.block_height;

            self.add_pair_prices(block_height, pair_price.created_at, vec![pair_price]);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        assertor::*,
        chrono::NaiveDateTime,
        grug::{NumberConst, Udec128_6, Udec128_24},
        std::{collections::VecDeque, str::FromStr},
    };

    #[tokio::test]
    async fn create_candles() -> Result<()> {
        let mut candle_cache = CandleCache::default();

        candle_cache.add_multi_block_pair_prices(parsed_pair_prices()?)?;

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
                0,
                0,
                "2025-08-13 17:36:01.038565",
                4,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                1217208030172232059779705322,
                0,
                0,
                "2025-08-13 17:36:00.086616",
                2,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                1216963512618701957116501833,
                345160391948092,
                420047603001999188,
                "2025-08-13 17:36:01.734583",
                6,
            )?,
        ];

        candle_cache.add_multi_block_pair_prices(pair_prices)?;

        let cache_key = CandleCacheKey::new(
            base_denom.to_string(),
            quote_denom.to_string(),
            CandleInterval::OneSecond,
        );

        assert_that!(candle_cache.get_candles(&cache_key).cloned().unwrap()).has_length(2);

        Ok(())
    }

    #[tokio::test]
    async fn close_price_is_correct() -> Result<()> {
        let mut candle_cache = CandleCache::default();

        let quote_denom = "bridge/usdc";
        let base_denom = "bridge/btc";

        let pair_prices = vec![
            pair_price(
                quote_denom,
                base_denom,
                27500000000000000000000000,
                345160391948092,
                420047603001999188,
                "1971-01-01 00:00:00.500",
                2,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                27500000000000000000000000,
                345160391948092,
                420047603001999188,
                "1971-01-01 00:00:01.000",
                4,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                25000000000000000000000000,
                345160391948092,
                420047603001999188,
                "1971-01-01 00:00:01.500",
                6,
            )?,
        ];

        candle_cache.add_multi_block_pair_prices(pair_prices)?;

        let cache_key = CandleCacheKey::new(
            base_denom.to_string(),
            quote_denom.to_string(),
            CandleInterval::OneSecond,
        );

        let candles = candle_cache.get_candles(&cache_key).cloned().unwrap();

        let first_candle = candles.first().unwrap();

        assert_that!(first_candle.open).is_equal_to(Udec128_24::from_str("27.5").unwrap());
        assert_that!(first_candle.close).is_equal_to(Udec128_24::from_str("27.5").unwrap());

        let last_candle = candles.last().unwrap();
        assert_that!(last_candle.open).is_equal_to(Udec128_24::from_str("27.5").unwrap());
        assert_that!(last_candle.close).is_equal_to(Udec128_24::from_str("25").unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn missing_pair_prices_creates_candles() -> Result<()> {
        let mut candle_cache = CandleCache::default();

        let quote_denom = "bridge/usdc";
        let base_denom = "bridge/btc";

        let pair_prices = vec![
            pair_price(
                quote_denom,
                base_denom,
                27500000000000000000000000,
                345160391948092,
                420047603001999188,
                "1971-01-01 00:00:00.500",
                2,
            )?,
            pair_price(
                quote_denom,
                base_denom,
                27500000000000000000000000,
                345160391948092,
                420047603001999188,
                "1971-01-01 00:00:01.500",
                4,
            )?,
        ];

        candle_cache.add_multi_block_pair_prices(pair_prices)?;

        candle_cache.add_pair_prices(3, parse_timestamp("1971-01-01 00:00:01.000")?, vec![]);
        candle_cache.add_pair_prices(5, parse_timestamp("1971-01-01 00:00:02.000")?, vec![]);

        let cache_key = CandleCacheKey::new(
            base_denom.to_string(),
            quote_denom.to_string(),
            CandleInterval::OneSecond,
        );

        let candles = candle_cache.get_candles(&cache_key).cloned().unwrap();

        assert_that!(candles).has_length(3);
        assert_that!(candles.iter().map(|c| c.blocks_count).sum()).is_equal_to(4);

        let last_candle = candles.last().unwrap();
        assert_that!(last_candle.open).is_equal_to(Udec128_24::from_str("27.5").unwrap());
        assert_that!(last_candle.close).is_equal_to(Udec128_24::from_str("27.5").unwrap());

        assert_that!(last_candle.volume_quote).is_equal_to(Udec128_6::ZERO);
        assert_that!(last_candle.volume_base).is_equal_to(Udec128_6::ZERO);

        Ok(())
    }

    fn parse_timestamp(s: &str) -> Result<DateTime<Utc>> {
        let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.3f")?;
        Ok(DateTime::from_naive_utc_and_offset(naive, Utc))
    }

    fn parsed_pair_prices() -> Result<Vec<PairPrice>> {
        let quote_denom = "bridge/usdc";
        let base_denom = "bridge/btc";

        Ok(vec![
            pair_price(
                quote_denom,
                base_denom,
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
                0,
                0,
                "2025-08-13 17:36:01.086616",
                1277885,
            )?,
            pair_price(
                quote_denom,
                base_denom,
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
                108404821869154,
                131924528945998920,
                "2025-08-13 17:36:03.182534",
                1277887,
            )?,
            pair_price(
                quote_denom,
                base_denom,
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
        clearing_price: u128,
        volume_base: u128,
        volume_quote: u128,
        created_at: &str,
        block_height: u64,
    ) -> Result<PairPrice> {
        Ok(PairPrice {
            quote_denom: quote_denom.to_string(),
            base_denom: base_denom.to_string(),
            clearing_price: Udec128_24::raw(grug::Int::new(clearing_price)),
            volume_base: Udec128_6::raw(grug::Int::new(volume_base)),
            volume_quote: Udec128_6::raw(grug::Int::new(volume_quote)),
            created_at: NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S%.f")?
                .and_utc(),
            block_height,
        })
    }
}
