use {
    crate::{
        context::Context,
        entities::{
            perps_candle::PerpsCandle, perps_candle_query::MAX_ITEMS,
            perps_pair_price::PerpsPairPrice,
        },
        error::Result,
    },
    chrono::{DateTime, Utc},
};

/// Take care of creating perps candles and storing them in clickhouse when needed
pub struct PerpsCandleGenerator {
    context: Context,
}

impl PerpsCandleGenerator {
    pub fn new(context: Context) -> Self {
        Self { context }
    }

    async fn store_candles(&self, candles: Vec<PerpsCandle>) -> Result<()> {
        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.perps_candles.stored.total")
            .increment(candles.len() as u64);

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} perps candles", candles.len());

        let mut inserter = self
            .context
            .clickhouse_client()
            .inserter::<PerpsCandle>("perps_candles")
            .with_max_rows(candles.len() as u64);

        for candle in candles {
            inserter.write(&candle).await.inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write perps candle: {candle:#?}: {_err}");
            })?;
        }

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserter for perps candles: {_err}");
        })?;

        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for perps candles: {_err}");
        })?;

        Ok(())
    }

    /// Saving all candles, even if not yet finished
    pub async fn save_all_candles(&self) -> Result<()> {
        let mut candle_cache = self.context.perps_candle_cache.write().await;
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
        pair_prices: Vec<PerpsPairPrice>,
    ) -> Result<()> {
        #[cfg(feature = "metrics")]
        metrics::counter!("indexer.clickhouse.perps_pair_prices.processed.total")
            .increment(pair_prices.len() as u64);

        #[cfg(feature = "tracing")]
        tracing::debug!("Saving {} perps pair prices", pair_prices.len());

        // Write pair prices to ClickHouse
        let mut inserter = self
            .context
            .clickhouse_client()
            .inserter::<PerpsPairPrice>("perps_pair_prices")
            .with_max_rows(pair_prices.len() as u64);

        for pair_price in pair_prices.iter() {
            inserter.write(pair_price).await.inspect_err(|_err| {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to write perps pair price: {pair_price:#?}: {_err}");
            })?;
        }

        inserter.commit().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to commit inserter for perps pair prices: {_err}");
        })?;
        inserter.end().await.inspect_err(|_err| {
            #[cfg(feature = "tracing")]
            tracing::error!("Failed to end inserter for perps pair prices: {_err}");
        })?;

        let mut candle_cache = self.context.perps_candle_cache.write().await;

        let _candles = candle_cache.add_pair_prices(block_height, created_at, pair_prices);

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
