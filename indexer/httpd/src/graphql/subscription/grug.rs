use {
    async_graphql::{futures_util::stream::Stream, *},
    futures_util::stream::{StreamExt, once},
    grug_httpd::{
        graphql::{
            query::grug::GrugQuery,
            types::{query_response::QueryResponseWithBlockHeight, status::Status, store::Store},
        },
        subscription_limiter::{acquire_subscription, guard_subscription_stream},
    },
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
            desc = "Receive updates every N blocks from the initial block height when subscription starts"
        )]
        block_interval: u64,
    ) -> Result<impl Stream<Item = Result<QueryResponseWithBlockHeight, Error>> + 'a> {
        let sub_guard = acquire_subscription(ctx)?;

        if block_interval == 0 {
            return Err(Error::new("blockInterval must be >= 1"));
        }

        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_app",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;
        let initial_response = GrugQuery::_query_app(&app_ctx.base, request.clone(), None).await?;
        let latest_block_height = initial_response.block_height;

        Ok(guard_subscription_stream(
            once({
                #[cfg(feature = "metrics")]
                let _guard = gauge_guard.clone();

                async move { Ok(initial_response) }
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
                    .then(move |_block_height| {
                        #[cfg(feature = "metrics")]
                        let _guard = gauge_guard.clone();
                        let request = request.clone();

                        async move { GrugQuery::_query_app(&app_ctx.base, request, None).await }
                    }),
            ),
            sub_guard,
        ))
    }

    async fn query_store<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(desc = "Key as B64 string")] key: String,
        #[graphql(default = false)] prove: bool,
        #[graphql(
            default = 10,
            desc = "Receive updates every N blocks from the initial block height when subscription starts"
        )]
        block_interval: u64,
    ) -> Result<impl Stream<Item = Result<Store, Error>> + 'a> {
        let sub_guard = acquire_subscription(ctx)?;

        if block_interval == 0 {
            return Err(Error::new("blockInterval must be >= 1"));
        }

        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        let gauge_guard = Arc::new(GaugeGuard::new(
            "graphql.subscriptions.active",
            "query_store",
            "subscription",
        ));

        let stream = app_ctx.pubsub.subscribe().await?;
        let initial_response =
            GrugQuery::_query_store(&app_ctx.base, key.clone(), None, prove).await?;
        let latest_block_height = initial_response.block_height;

        Ok(guard_subscription_stream(once({
            #[cfg(feature = "metrics")]
            let _guard = gauge_guard.clone();

            async { Ok(initial_response) }
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
                .then(move |_block_height| {
                    #[cfg(feature = "metrics")]
                    let _guard = gauge_guard.clone();
                    let key = key.clone();

                    async move { GrugQuery::_query_store(&app_ctx.base, key, None, prove).await }
                }),
        ), sub_guard))
    }

    async fn query_status<'a>(
        &self,
        ctx: &async_graphql::Context<'a>,
        #[graphql(
            default = 10,
            desc = "Receive updates every N blocks from the initial block height when subscription starts"
        )]
        block_interval: u64,
    ) -> Result<impl Stream<Item = Result<Status, Error>> + 'a> {
        let sub_guard = acquire_subscription(ctx)?;

        if block_interval == 0 {
            return Err(Error::new("blockInterval must be >= 1"));
        }

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

        Ok(guard_subscription_stream(
            once({
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
            ),
            sub_guard,
        ))
    }
}
