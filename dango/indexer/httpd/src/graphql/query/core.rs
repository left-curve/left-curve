use {
    crate::{
        context::MinimalContext,
        graphql::types::{
            query_response::QueryResponseWithBlockHeight, status::Status, store::Store,
        },
        request_ip::RequesterIp,
    },
    async_graphql::*,
    dango_primitives::{Binary, Inner, Query, QueryResponse, TxOutcome, UnsignedTx},
    std::str::FromStr,
};
#[cfg(feature = "metrics")]
use {metrics::histogram, std::time::Instant};

#[derive(Default, Debug)]
pub struct CoreQuery {}

impl CoreQuery {
    pub async fn _query_app(
        app_ctx: &MinimalContext,
        request: Query,
        height: Option<u64>,
    ) -> Result<QueryResponseWithBlockHeight, Error> {
        #[cfg(feature = "metrics")]
        let start = Instant::now();

        let (response, block_height) = app_ctx.dango_app.query_app(request, height).await?;

        #[cfg(feature = "metrics")]
        histogram!("http.grug.query_app.duration").record(start.elapsed().as_secs_f64());

        Ok(QueryResponseWithBlockHeight {
            response,
            block_height,
        })
    }

    pub async fn _query_store(
        app_ctx: &MinimalContext,
        key: String,
        height: Option<u64>,
        prove: bool,
    ) -> Result<Store, Error> {
        let key = Binary::from_str(&key)?;

        #[cfg(feature = "metrics")]
        let start = Instant::now();

        let (value, proof, block_height) = app_ctx
            .dango_app
            .query_store(key.inner(), height, prove)
            .await?;

        #[cfg(feature = "metrics")]
        histogram!("http.grug.query_store.duration").record(start.elapsed().as_secs_f64());

        let value = if let Some(value) = value {
            Binary::from(value).to_string()
        } else {
            return Err(Error::new(format!("Key not found: {key}")));
        };

        Ok(Store {
            value,
            proof: proof.map(|proof| Binary::from(proof).to_string()),
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
        let app_ctx = ctx.data::<MinimalContext>()?;

        Self::_query_app(app_ctx, request, height)
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
