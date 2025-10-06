use {
    crate::entities::trade::Trade,
    async_graphql::{futures_util::stream::Stream, *},
    dango_types::dex::PairId,
    futures::stream,
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
        let trade_cache = app_ctx.trade_cache.clone();

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "trades",
            "subscription",
        ));

        let pair = PairId {
            base_denom: base_denom.parse()?,
            quote_denom: quote_denom.parse()?,
        };

        // connect to the pubsub first, to avoid missing data.
        let stream = app_ctx.trade_pubsub.subscribe().await?;
        let initial_trades = trade_cache
            .read()
            .await
            .trades_for_pair(&pair)
            .cloned()
            .unwrap_or_default();

        Ok(
            stream::iter(initial_trades).chain(stream.filter_map(move |trade| {
                #[cfg(feature = "metrics")]
                let _guard = gauge_guard.clone();

                let quote_denom = quote_denom.clone();
                let base_denom = base_denom.clone();

                async move {
                    if trade.quote_denom == quote_denom && trade.base_denom == base_denom {
                        Some(trade)
                    } else {
                        None
                    }
                }
            })),
        )
    }
}
