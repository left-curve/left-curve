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
            Ok(candle_cache
                .read()
                .await
                .get_last_candle(&cache_key)
                .map(|candle| vec![candle.clone()])
                .unwrap_or_default())
        })
        .chain(
            app_ctx
                .candle_pubsub
                .subscribe_candles_cached()
                .await?
                .then(move |_block_height| {
                    let cache_key = cache::CandleCacheKey::new(
                        base_denom.clone(),
                        quote_denom.clone(),
                        interval,
                    );
                    let candle_cache = app_ctx.candle_cache.clone();

                    async move {
                        Ok(candle_cache
                            .read()
                            .await
                            .get_last_candle(&cache_key)
                            .map(|candle| vec![candle.clone()])
                            .unwrap_or_default())
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
