#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;
use {
    crate::{
        entities::{CandleInterval, perps_candle::PerpsCandle},
        indexer::perps_candles::cache,
    },
    async_graphql::{futures_util::stream::Stream, *},
    chrono::{DateTime, Utc},
    futures_util::stream::{StreamExt, once},
    grug_httpd::subscription_limiter::{acquire_subscription, guard_subscription_stream},
    std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Default)]
pub struct PerpsCandleSubscription;

#[Subscription]
impl PerpsCandleSubscription {
    /// Get perps candles for a given pair_id and interval
    async fn perps_candles<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        pair_id: String,
        interval: CandleInterval,
        #[allow(unused_variables)]
        #[graphql(deprecation)]
        later_than: Option<DateTime<Utc>>,
        #[allow(unused_variables)]
        #[graphql(deprecation)]
        limit: Option<usize>,
    ) -> Result<impl Stream<Item = Vec<PerpsCandle>> + 'a> {
        let sub_guard = acquire_subscription(ctx)?;
        let app_ctx = ctx.data::<crate::context::Context>()?;
        let candle_cache = app_ctx.perps_candle_cache.clone();
        let cache_key = cache::PerpsCandleCacheKey::new(pair_id.clone(), interval);

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "perps_candles",
            "subscription",
        ));

        let received_block_height = Arc::new(AtomicU64::new(0));
        // connect to the pubsub first, to avoid missing data.
        let stream = app_ctx.pubsub.subscribe().await?;
        let initial_candle = candle_cache
            .read()
            .await
            .get_last_candle(&cache_key)
            .cloned();

        Ok(guard_subscription_stream(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move { Ok(initial_candle) }
        })
        .chain(stream.then(move |current_block_height| {
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            let cache_key = cache::PerpsCandleCacheKey::new(pair_id.clone(), interval);
            let candle_cache = app_ctx.perps_candle_cache.clone();

            let previous_block_height =
                received_block_height.fetch_max(current_block_height, Ordering::Release);

            async move {
                if current_block_height < previous_block_height {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        previous_block_height,
                        current_block_height,
                        "Skip perps candle, pubsub block_height is lower than previous"
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
                    tracing::info!(
                        previous_block_height,
                        current_block_height,
                        "No current perps candle, expected if chain started"
                    );
                    return Ok(None);
                };

                if candle.max_block_height < current_block_height {
                    #[cfg(feature = "tracing")]
                    tracing::warn!(
                        previous_block_height,
                        current_block_height,
                        %candle.max_block_height,
                        interval = ?interval,
                        "Skip perps candle, it has older block_height than received pubsub block_height"
                    );

                    return Ok(None);
                }

                Ok(Some(candle))
            }
        }))
        .filter_map(|candle: Result<Option<PerpsCandle>>| async move {
            match candle {
                Ok(Some(candle)) => Some(vec![candle]),
                Ok(None) => None,
                Err(_err) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("Error getting perps candles: {_err:?}");

                    None
                },
            }
        }), sub_guard))
    }
}
