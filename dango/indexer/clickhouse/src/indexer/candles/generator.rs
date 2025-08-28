use chrono::{DateTime, Utc};

use crate::{
    context::Context,
    entities::{candle::Candle, candle_query::MAX_ITEMS, pair_price::PairPrice},
    error::Result,
};

/// Take care of creating candles and storing them in clickhouse when needed
pub struct CandleGenerator {
    context: Context,
}

impl CandleGenerator {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    async fn store_candles(&self, candles: Vec<Candle>) -> Result<()> {
        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.candles.stored.total")
            .increment(candles.len() as u64);

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} candles", candles.len());

        let mut inserter = self
            .context
            .clickhouse_client()
            .inserter::<Candle>("candles")?
            .with_max_rows(candles.len() as u64);

        for candle in candles {
            inserter.write(&candle).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write candle: {candle:#?}: {_err}");
            })?;
        }

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserter for candles: {_err}");
        })?;
        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for candles: {_err}");
        })?;

        Ok(())
    }

    /// Saving all candles, even if not yet finished
    pub async fn save_all_candles(&self) -> Result<()> {
        let mut candle_cache = self.context.candle_cache.write().await;
        let mut candles = candle_cache.completed_candles.drain().collect::<Vec<_>>();

        for candle_list in candle_cache.candles.values() {
            candles.extend(candle_list.clone());
        }

        drop(candle_cache);

        self.store_candles(candles).await
    }

    pub async fn add_pair_prices(
        &self,
        block_height: u64,
        created_at: DateTime<Utc>,
        pair_prices: Vec<PairPrice>,
    ) -> Result<()> {
        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.pair_prices.processed.total")
            .increment(pair_prices.len() as u64);

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} pair prices", pair_prices.len());

        // Use Row binary inserter with the official clickhouse serde helpers
        let mut inserter = self
            .context
            .clickhouse_client()
            .inserter::<PairPrice>("pair_prices")?
            .with_max_rows(pair_prices.len() as u64);

        for pair_price in pair_prices.iter() {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "{} Inserting pair price: {}",
                pair_price.block_height,
                pair_price.clearing_price,
            );

            inserter.write(pair_price).inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write pair price: {pair_price:#?}: {_err}");
            })?;
        }

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserter for pair prices: {_err}");
        })?;
        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for pair prices: {_err}");
        })?;

        let mut candle_cache = self.context.candle_cache.write().await;

        // Use this to store all candles, meaning they're going to be saved multiple times
        // and generate duplicates with clickhouse.
        let _candles = candle_cache.add_pair_prices(block_height, created_at, pair_prices);
        // Use this to be smarter and only store them once, once completed.
        // NOTE: I only want to save past candles when they are complete and there is no gaps in pair_price
        let mut candles = Vec::new();

        if !candle_cache.completed_candles.is_empty() && !candle_cache.has_gaps() {
            candles = candle_cache.completed_candles.drain().collect::<Vec<_>>();
        }

        candle_cache.compact_keep_n(MAX_ITEMS * 2);
        drop(candle_cache);

        self.store_candles(candles).await?;

        Ok(())
    }
}
