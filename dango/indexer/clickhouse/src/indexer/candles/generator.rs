use chrono::{DateTime, Utc};

use crate::{
    context::Context,
    entities::{candle::Candle, candle_query::MAX_ITEMS, pair_price::PairPrice},
    error::Result,
};

pub struct CandleGenerator {
    context: Context,
}

impl CandleGenerator {
    pub fn new(context: Context) -> Self {
        Self { context }
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
        let candles = candle_cache.add_pair_prices(block_height, created_at, pair_prices);
        candle_cache.compact_keep_n(MAX_ITEMS * 2);
        drop(candle_cache);

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
}
