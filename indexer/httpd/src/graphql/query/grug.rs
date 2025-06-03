#[cfg(feature = "metrics")]
use metrics::counter;
use {
    super::super::types::status::Status, crate::graphql::types::store::Store, async_graphql::*,
    grug_math::Inner, grug_types::Binary, std::str::FromStr,
};

#[derive(Default, Debug)]
pub struct GrugQuery {}

#[Object]
impl GrugQuery {
    async fn query_app(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Request as JSON string")] request: String,
        height: Option<u64>,
    ) -> Result<String, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        counter!("graphql.grug.query_app.calls").increment(1);

        Ok(app_ctx.grug_app.query_app(request, height).await?)
    }

    async fn query_store(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Key as B64 string")] key: String,
        height: Option<u64>,
        #[graphql(default = false)] prove: bool,
    ) -> Result<Store, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;
        let key = Binary::from_str(&key)?;

        let (value, proof) = app_ctx
            .grug_app
            .query_store(key.inner(), height, prove)
            .await?;

        let value = if let Some(value) = value {
            Binary::from(value).to_string()
        } else {
            return Err(Error::new(format!("Key not found: {}", key)));
        };

        #[cfg(feature = "metrics")]
        counter!("graphql.grug.query_store.calls").increment(1);

        Ok(Store {
            value,
            proof: proof.map(|proof| Binary::from(proof).to_string()),
        })
    }

    async fn query_status(&self, ctx: &async_graphql::Context<'_>) -> Result<Status, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let status = Status {
            block: app_ctx.grug_app.last_finalized_block().await?.into(),
            chain_id: app_ctx.grug_app.chain_id().await?,
        };

        #[cfg(feature = "metrics")]
        counter!("graphql.grug.query_status.calls").increment(1);

        Ok(status)
    }

    async fn simulate(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Transaction as Json string")] tx: String,
    ) -> Result<String, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        #[cfg(feature = "metrics")]
        counter!("graphql.grug.simulate.calls").increment(1);

        Ok(app_ctx.grug_app.simulate(tx).await?)
    }
}
