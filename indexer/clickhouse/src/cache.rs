use {
    crate::entities::{CandleInterval, candle::Candle, candle_query::CandleQueryBuilder},
    std::collections::HashMap,
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
    /// If the candle is found in the cache and the block height is not greater
    /// than the last candle's block height, returns the last candle.
    /// If not found, fetches the latest candle from ClickHouse and
    /// saves it in the cache, returning it.
    pub async fn get_or_save_new_candle(
        &mut self,
        key: CandleCacheKey,
        clickhouse_client: &clickhouse::Client,
        block_height: Option<u64>,
    ) -> Result<Option<Candle>, crate::error::IndexerError> {
        let cached_candle = self.candles.get(&key).and_then(|c| c.last());

        // If found in cache and the cache block_height is at least block_height,
        match (cached_candle, block_height) {
            (Some(cached_candle), Some(block_height))
                if cached_candle.block_height >= block_height =>
            {
                return Ok(Some(cached_candle.clone()));
            },
            (Some(cached_candle), None) => {
                return Ok(Some(cached_candle.clone()));
            },
            _ => {},
        }

        // Not found in cache, fetch from ClickHouse
        let query_builder = CandleQueryBuilder::new(
            key.interval,
            key.base_denom.clone(),
            key.quote_denom.clone(),
        );

        // if let Some(block_height) = block_height {
        //     query_builder = query_builder.after_block_height(block_height);
        // }

        // No candle for this pair and interval
        let Some(fetched_candle) = query_builder.fetch_one(clickhouse_client).await? else {
            return Ok(None);
        };

        // Cache the candle only if the fetched candle is newer
        if cached_candle.map_or(true, |cached_candle| {
            cached_candle.block_height < fetched_candle.block_height
        }) {
            self.add_candle(key.clone(), fetched_candle.clone());
        }

        // If the fetched candle's block height is less than the provided block height,
        if let Some(block_height) = block_height {
            if fetched_candle.block_height < block_height {
                return Ok(None);
            }
        }

        Ok(Some(fetched_candle.clone()))
    }

    pub fn add_candle(&mut self, key: CandleCacheKey, candle: Candle) {
        self.candles.entry(key).or_default().push(candle);
    }

    pub fn get_candles(&self, key: &CandleCacheKey) -> Option<&Vec<Candle>> {
        self.candles.get(key)
    }

    pub fn get_last_candle(&self, key: &CandleCacheKey) -> Option<&Candle> {
        self.candles.get(key).and_then(|candles| candles.last())
    }

    // Keep only the most recent candle
    pub fn compact(&mut self) {
        self.candles.retain(|_key, candles| {
            if candles.is_empty() {
                false
            } else {
                // Keep only the LAST (most recent) candle
                if let Some(last_candle) = candles.pop() {
                    candles.clear();
                    candles.push(last_candle);
                }
                true
            }
        });
    }

    // Keep only the most recent candle
    pub fn compact_for_key(&mut self, key: &CandleCacheKey) {
        if let Some(candles) = self.candles.get_mut(key) {
            if !candles.is_empty() {
                // Keep only the LAST (most recent) candle
                if let Some(last_candle) = candles.pop() {
                    candles.clear();
                    candles.push(last_candle);
                }
            }
        }
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
