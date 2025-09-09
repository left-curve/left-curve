use {
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    grug_httpd::graphql::{
        query::grug::GrugQuery,
        types::{status::Status, store::Store},
    },
    grug_types::QueryResponse,
};
#[cfg(feature = "metrics")]
use {grug_httpd::metrics::GaugeGuard, std::sync::Arc};

#[derive(Default)]
pub struct GrugSubscription;

#[Subscription]
impl GrugSubscription {
    async fn query_app<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(desc = "Request as JSON")] request: grug_types::Query,
    ) -> Result<impl Stream<Item = Result<QueryResponse, Error>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_app",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();
            let request = request.clone();

            async { GrugQuery::_query_app(&app_ctx.base, request, None).await }
        })
        .chain(stream.then(move |block_height| {
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();
            let request = request.clone();

            async move { GrugQuery::_query_app(&app_ctx.base, request, Some(block_height)).await }
        })))
        // .filter_map(|query_response| async move { query_response.ok() }))
    }

    async fn query_store<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(desc = "Key as B64 string")] key: String,
        #[graphql(default = false)] prove: bool,
    ) -> Result<impl Stream<Item = Result<Store, Error>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_store",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();
            let key = key.clone();

            async move { GrugQuery::_query_store(&app_ctx.base, key, None, prove).await }
        })
        .chain(
            stream.then(move |block_height| {
                #[cfg(feature = "metrics")]
                let _guard = gauge_guard.clone();
                let key = key.clone();

                async move {
                    GrugQuery::_query_store(&app_ctx.base, key, Some(block_height), prove).await
                }
            }),
        ))
    }

    async fn query_status<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
    ) -> Result<impl Stream<Item = Result<Status, Error>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_status",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move { GrugQuery::_query_status(&app_ctx.base).await }
        })
        .chain(stream.then(move |_| {
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async move { GrugQuery::_query_status(&app_ctx.base).await }
        })))
    }
}
