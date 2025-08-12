use {
    crate::{
        Indexer,
        entities::{CandleInterval, pair_price::PairPrice},
        error::IndexerError,
    },
    strum::IntoEnumIterator,
};

impl Indexer {
    /// Will look at every candle and check the open price is the same as previous candle close
    pub async fn check_all(&self) -> Result<bool, IndexerError> {
        let pairs = PairPrice::all_pairs(self.context.clickhouse_client()).await?;

        for pair in pairs {
            for interval in CandleInterval::iter() {
                let base_denom = pair.base_denom.to_string();
                let quote_denom = pair.quote_denom.to_string();

                #[cfg(feature = "tracing")]
                tracing::info!("Checking candles for {base_denom}/{quote_denom} at {interval:?}");

                if !self.check(&base_denom, &quote_denom, interval).await? {
                    #[cfg(feature = "tracing")]
                    tracing::info!("candles don't match");

                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    async fn check(
        &self,
        base_denom: &str,
        quote_denom: &str,
        interval: CandleInterval,
    ) -> Result<bool, IndexerError> {
        #[cfg(feature = "tracing")]
        tracing::info!("Checking candles for {base_denom}/{quote_denom} at {interval:?}");

        let clickhouse_client = self.context.clickhouse_client().clone();
        let query_builder = crate::entities::candle_query::CandleQueryBuilder::new(
            interval,
            base_denom.to_string(),
            quote_denom.to_string(),
        );

        // will get most recent candle first
        let mut cursor = query_builder.fetch(&clickhouse_client)?;

        let Some(mut later_candle) = cursor.next().await? else {
            #[cfg(feature = "tracing")]
            tracing::info!("No candle found");
            return Ok(true);
        };

        let mut error = false;

        while let Some(candle) = cursor.next().await? {
            if candle.close != later_candle.open {
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    ?candle,
                    ?later_candle,
                    candle_close=?candle.close,
                    later_candle_open=?later_candle.open,
                    block_height=candle.block_height,
                    "Candle close price does not match later candle close"
                );
                error = true;
            }

            later_candle = candle;
        }

        #[cfg(feature = "tracing")]
        tracing::info!("All candles checked successfully");

        Ok(!error)
    }
}
