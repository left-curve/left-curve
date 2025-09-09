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
        #[graphql(
            default = 10,
            desc = "Receive updates every N blocks (e.g., 10 = update every 10th block from the latest)"
        )]
        block_interval: u64,
    ) -> Result<impl Stream<Item = Result<QueryResponse, Error>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_app",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;
        let initial_response = GrugQuery::_query_app(&app_ctx.base, request.clone(), None).await;
        let latest_block_height = app_ctx.base.grug_app.last_finalized_block().await?.height;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async { initial_response }
        })
        .chain(
            stream
                .scan(latest_block_height, move |last_processed, block_height| {
                    let result = if block_height > *last_processed
                        && (block_height - latest_block_height) % block_interval == 0
                    {
                        *last_processed = block_height;
                        Some(Some(block_height))
                    } else {
                        Some(None)
                    };
                    futures::future::ready(result)
                })
                .filter_map(|opt_height| async move { opt_height })
                .then(move |block_height| {
                    #[cfg(feature = "metrics")]
                    let _guard = gauge_guard.clone();
                    let request = request.clone();

                    async move {
                        GrugQuery::_query_app(&app_ctx.base, request, Some(block_height)).await
                    }
                }),
        ))
    }

    async fn query_store<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(desc = "Key as B64 string")] key: String,
        #[graphql(default = false)] prove: bool,
        #[graphql(
            default = 10,
            desc = "Receive updates every N blocks (e.g., 10 = update every 10th block from the latest)"
        )]
        block_interval: u64,
    ) -> Result<impl Stream<Item = Result<Store, Error>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_store",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;
        let initial_response =
            GrugQuery::_query_store(&app_ctx.base, key.clone(), None, prove).await;
        let latest_block_height = app_ctx.base.grug_app.last_finalized_block().await?.height;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async { initial_response }
        })
        .chain(
            stream
                .scan(latest_block_height, move |last_processed, block_height| {
                    let result = if block_height > *last_processed
                        && (block_height - latest_block_height) % block_interval == 0
                    {
                        *last_processed = block_height;
                        Some(Some(block_height))
                    } else {
                        Some(None)
                    };
                    futures::future::ready(result)
                })
                .filter_map(|opt_height| async move { opt_height })
                .then(move |block_height| {
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
        #[graphql(
            default = 10,
            desc = "Receive updates every N blocks (e.g., 10 = update every 10th block from the latest)"
        )]
        block_interval: u64,
    ) -> Result<impl Stream<Item = Result<Status, Error>> + 'a> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_status",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;
        let initial_response = GrugQuery::_query_status(&app_ctx.base).await;
        let latest_block_height = app_ctx.base.grug_app.last_finalized_block().await?.height;

        Ok(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async { initial_response }
        })
        .chain(
            stream
                .scan(latest_block_height, move |last_processed, block_height| {
                    let result = if block_height > *last_processed
                        && (block_height - latest_block_height) % block_interval == 0
                    {
                        *last_processed = block_height;
                        Some(Some(block_height))
                    } else {
                        Some(None)
                    };
                    futures::future::ready(result)
                })
                .filter_map(|opt_height| async move { opt_height })
                .then(move |_| {
                    #[cfg(feature = "metrics")]
                    let _guard = gauge_guard.clone();

                    async { GrugQuery::_query_status(&app_ctx.base).await }
                }),
        ))
    }
}
