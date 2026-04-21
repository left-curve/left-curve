#[cfg(feature = "metrics")]
use grug_httpd::metrics::GaugeGuard;
use {
    crate::entities::pair_stats::PairStats,
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    grug_httpd::subscription_limiter::{acquire_subscription, guard_subscription_stream},
    std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Default)]
pub struct PairStatsSubscription;

#[Subscription]
impl PairStatsSubscription {
    /// Stream real-time 24h statistics for all trading pairs.
    ///
    /// Returns the current cached snapshot immediately, then emits an updated
    /// snapshot each time a new block is indexed.
    async fn all_pair_stats<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
    ) -> Result<impl Stream<Item = Vec<PairStats>> + 'a> {
        let sub_guard = acquire_subscription(ctx)?;
        let app_ctx = ctx.data::<crate::context::Context>()?;
        let cache = app_ctx.pair_stats_cache.clone();

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "all_pair_stats",
            "subscription",
        ));

        let received_block_height = Arc::new(AtomicU64::new(0));
        // Subscribe to block-height notifications first to avoid missing data.
        let stream = app_ctx.pubsub.subscribe().await?;
        let initial = cache.read().await.stats().to_vec();

        Ok(guard_subscription_stream(
            once({
                #[cfg(feature = "metrics")]
                let _guard = gauge_guard.clone();

                async move { initial }
            })
            .chain(stream.filter_map(move |current_block_height| {
                #[cfg(feature = "metrics")]
                let _guard = gauge_guard.clone();

                let cache = cache.clone();
                let previous_block_height =
                    received_block_height.fetch_max(current_block_height, Ordering::Release);

                async move {
                    if current_block_height < previous_block_height {
                        return None;
                    }

                    let stats = cache.read().await.stats().to_vec();
                    Some(stats)
                }
            })),
            sub_guard,
        ))
    }
}
