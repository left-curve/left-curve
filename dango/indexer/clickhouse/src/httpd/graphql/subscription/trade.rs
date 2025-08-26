use {
    crate::entities::trade::Trade,
    async_graphql::{futures_util::stream::Stream, *},
    futures::future::ready,
    futures_util::stream::StreamExt,
};
#[cfg(feature = "metrics")]
use {grug_httpd::metrics::GaugeGuard, std::sync::Arc};

#[derive(Default)]
pub struct TradeSubscription;

#[Subscription]
impl TradeSubscription {
    /// Get all trades, this will not include past trades but only new ones since you subscribed.
    async fn trades<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        base_denom: String,
        quote_denom: String,
    ) -> Result<impl Stream<Item = Trade> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "trades",
            "subscription",
        ));

        Ok(app_ctx
            .trade_pubsub
            .subscribe()
            .await?
            .filter(move |trade| {
                #[cfg(feature = "metrics")]
                let _guard = gauge_guard.clone();

                ready(trade.quote_denom == quote_denom && trade.base_denom == base_denom)
            }))
    }
}
