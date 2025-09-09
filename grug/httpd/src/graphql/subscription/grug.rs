#[cfg(feature = "metrics")]
use {crate::metrics::GaugeGuard, std::sync::Arc};
use {
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    grug_types::QueryResponse,
};

#[derive(Default)]
pub struct GrugSubscription;

#[Subscription]
impl GrugSubscription {
    async fn query_app<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(desc = "Request as JSON")] request: grug_types::Query,
    ) -> Result<impl Stream<Item = QueryResponse> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_app",
            "subscription",
        ));

        // let stream = app_ctx.pubsub.subscribe().await?;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async {  app_ctx.grug_app.query_app(request, None).await }
        })
        // .chain(app_ctx.pubsub.subscribe().await?.then(move |block_height| {
        //     let u = username.clone();

        //     #[cfg(feature = "metrics")]
        //     let _guard = gauge_guard.clone();

        //     async move {
        //         Self::get_accounts(app_ctx, block_height as i64..=block_height as i64, u).await
        //     }
        // }))
        .filter_map(|query_response| async move {
            query_response.ok()
        }))
    }
}
