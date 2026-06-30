use {
    crate::{
        context::MinimalContext,
        graphql::types::{query_response::QueryResponseWithBlockHeight, status::Status},
        request_ip::RequesterIp,
    },
    async_graphql::*,
    dango_primitives::{Query, QueryResponse, TxOutcome, UnsignedTx},
};
#[cfg(feature = "metrics")]
use {metrics::histogram, std::time::Instant};

#[derive(Default, Debug)]
pub struct CoreQuery {}

impl CoreQuery {
    pub async fn _query_app(
        app_ctx: &MinimalContext,
        request: Query,
    ) -> Result<QueryResponseWithBlockHeight, Error> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        let (response, block_height) = app_ctx.dango_app.query_app(request).await?;

        #[cfg(feature = "metrics")]
        histogram!("http.grug.query_app.duration").record(start.elapsed().as_secs_f64());

        Ok(QueryResponseWithBlockHeight {
            response,
            block_height,
        })
    }

    pub async fn _query_status(app_ctx: &MinimalContext) -> Result<Status, Error> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        let status = Status {
            block: app_ctx.dango_app.last_finalized_block().await?.into(),
            chain_id: app_ctx.dango_app.chain_id().await?,
        };

        #[cfg(feature = "metrics")]
        histogram!("http.grug.query_status.duration").record(start.elapsed().as_secs_f64());

        Ok(status)
    }
}

#[Object]
impl CoreQuery {
    async fn query_app(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Request as JSON")] request: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Error> {
        // Historical queries are not supported; only the latest finalized block
        // can be queried. Reject a non-`None` height explicitly rather than
        // silently ignoring it.
        if height.is_some() {
            return Err(Error::new("non-None `height` is not supported"));
        }

        let app_ctx = ctx.data::<MinimalContext>()?;

        Self::_query_app(app_ctx, request)
            .await
            .map(|res| res.response)
    }

    async fn query_status(&self, ctx: &async_graphql::Context<'_>) -> Result<Status, Error> {
        let app_ctx = ctx.data::<MinimalContext>()?;

        Self::_query_status(app_ctx).await
    }

    async fn requester_ip(&self, ctx: &async_graphql::Context<'_>) -> Result<RequesterIp, Error> {
        ctx.data::<RequesterIp>()
            .cloned()
            .map_err(|_| Error::new("requester_ip is only available on HTTP GraphQL requests"))
    }

    async fn simulate(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Transaction as Json")] tx: UnsignedTx,
    ) -> Result<TxOutcome, Error> {
        let app_ctx = ctx.data::<MinimalContext>()?;

        Ok(app_ctx.dango_app.simulate(tx).await?)
    }
}
