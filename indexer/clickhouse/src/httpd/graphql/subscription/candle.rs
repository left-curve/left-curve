use {
    crate::{
        cache,
        entities::{CandleInterval, candle::Candle},
    },
    async_graphql::{futures_util::stream::Stream, *},
    chrono::{DateTime, Utc},
    futures_util::stream::{StreamExt, once},
};

#[derive(Default)]
pub struct CandleSubscription;

#[Subscription]
impl CandleSubscription {
    /// Get candles for a given base and quote denom, interval
    async fn candles<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        base_denom: String,
        quote_denom: String,
        interval: CandleInterval,
        #[allow(unused_variables)]
        #[graphql(deprecation)]
        later_than: Option<DateTime<Utc>>,
        #[allow(unused_variables)]
        #[graphql(deprecation)]
        limit: Option<usize>,
    ) -> Result<impl Stream<Item = Vec<Candle>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;
        let candle_cache = app_ctx.candle_cache.clone();
        let cache_key =
            cache::CandleCacheKey::new(base_denom.clone(), quote_denom.clone(), interval);

        Ok(once(async move {
            // We first check the cache for the last candle since this will
            // always match except at the start of the httpd process, avoid unnecessary
            // write lock.
            if let Some(candle_cache) = candle_cache.read().await.get_last_candle(&cache_key) {
                return Ok(vec![candle_cache.clone()]);
            }

            if let Some(candle) = candle_cache
                .write()
                .await
                .get_or_save_new_candle(cache_key, app_ctx.clickhouse_client(), None)
                .await?
            {
                return Ok(vec![candle]);
            }

            Ok(vec![])
        })
        .chain(
            app_ctx
                .pubsub
                .subscribe_block_minted()
                .await?
                .then(move |block_height| {
                    let cache_key = cache::CandleCacheKey::new(
                        base_denom.clone(),
                        quote_denom.clone(),
                        interval,
                    );
                    let candle_cache = app_ctx.candle_cache.clone();

                    async move {
                        let mut candle_cache = candle_cache.write().await;

                        if let Some(candle) = candle_cache
                            .get_or_save_new_candle(
                                cache_key.clone(),
                                app_ctx.clickhouse_client(),
                                Some(block_height),
                            )
                            .await?
                        {
                            candle_cache.compact_for_key(&cache_key);

                            return Ok(vec![candle]);
                        }

                        Ok(vec![])
                    }
                }),
        )
        .filter_map(|candles: Result<Vec<Candle>>| async move {
            match candles {
                Ok(candles) => {
                    if candles.is_empty() {
                        None
                    } else {
                        Some(candles)
                    }
                },
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Error getting candles: {_err:?}");

                    None
                },
            }
        }))
    }
}
