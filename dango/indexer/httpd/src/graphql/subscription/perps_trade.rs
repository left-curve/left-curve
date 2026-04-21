use {
    async_graphql::{futures_util::stream::Stream, *},
    dango_indexer_sql::entity::perps_trade::PerpsTrade,
    futures_util::stream::{self, StreamExt},
    grug_httpd::subscription_limiter::{acquire_subscription, guard_subscription_stream},
};
#[cfg(feature = "metrics")]
use {grug_httpd::metrics::GaugeGuard, std::sync::Arc};

#[derive(Default)]
pub struct PerpsTradeSubscription;

#[Subscription]
impl PerpsTradeSubscription {
    /// Stream real-time perps trades for a given pair.
    /// Returns cached recent trades first, then streams new trades as they occur.
    async fn perps_trades<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(name = "pairId")] pair_id: String,
    ) -> Result<impl Stream<Item = PerpsTrade> + 'a> {
        let sub_guard = acquire_subscription(ctx)?;
        let app_ctx = ctx.data::<crate::context::Context>()?;
        let trade_cache = app_ctx.perps_trade_cache.clone();

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "perps_trades",
            "subscription",
        ));

        // Connect to the pubsub first to avoid missing data between cache read
        // and stream start.
        let pubsub_stream = app_ctx.perps_trade_pubsub.subscribe().await?;
        let initial_trades = trade_cache
            .read()
            .await
            .trades_for_pair(&pair_id)
            .cloned()
            .unwrap_or_default();

        Ok(guard_subscription_stream(
            stream::iter(initial_trades).chain(pubsub_stream.filter_map(move |trade| {
                #[cfg(feature = "metrics")]
                let _guard = gauge_guard.clone();

                let pair_id = pair_id.clone();

                async move {
                    if trade.pair_id == pair_id {
                        Some(trade)
                    } else {
                        None
                    }
                }
            })),
            sub_guard,
        ))
    }
}
