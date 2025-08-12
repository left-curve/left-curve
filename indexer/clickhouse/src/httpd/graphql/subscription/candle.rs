#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;

use {
    crate::{
        cache,
        entities::{CandleInterval, candle::Candle},
    },
    async_graphql::{futures_util::stream::Stream, *},
    chrono::{DateTime, Utc},
    futures_util::stream::{StreamExt, once},
    std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
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

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "candles",
            "subscription",
        ));

        let received_block_height = Arc::new(AtomicU64::new(0));

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move {
                Ok(candle_cache
                    .read()
                    .await
                    .get_last_candle(&cache_key)
                    .cloned())
            }
        })
        .chain(app_ctx.pubsub.subscribe().await?.then(move |block_height| {
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            let cache_key =
                cache::CandleCacheKey::new(base_denom.clone(), quote_denom.clone(), interval);
            let candle_cache = app_ctx.candle_cache.clone();

            let current_received = received_block_height.fetch_max(block_height, Ordering::Release);

            async move {
                if block_height < current_received {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        current_received,
                        block_height,
                        "Skip candle, pubsub block_height is lower than already received, shouldn't happen..."
                    );
                    return Ok(None);
                }

                let last_candle = candle_cache
                    .read()
                    .await
                    .get_last_candle(&cache_key)
                    .cloned();

                let Some(candle) = last_candle else {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        current_received,
                        block_height,
                        "No current candle, shouldn't happen..."
                    );
                    return Ok(None);
                };

                if candle.block_height < block_height {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        current_received,
                        block_height,
                        %candle.block_height,
                        "Skip candle, it has older block_height than pubsub received, shouldn't happen..."
                    );
                    return Ok(None);
                }


                Ok(Some(candle))
            }
        }))
        .filter_map(|candle: Result<Option<Candle>>| async move {
            match candle {
                Ok(Some(candle)) => {
                    Some(vec![candle])
                },
                Ok(None) => None,
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Error getting candles: {_err:?}");

                    None
                },
            }
        }))
    }
}
