use {
    super::super::types::status::Status,
    crate::graphql::types::store::Store,
    async_graphql::*,
    grug_math::Inner,
    grug_types::{Binary, JsonSerExt},
    std::str::FromStr,
};

#[derive(Default, Debug)]
pub struct GrugQuery {}

#[Object]
impl GrugQuery {
    async fn query_app(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Request as JSON")] request: serde_json::Value,
        height: Option<u64>,
    ) -> Result<serde_json::Value, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        Ok(app_ctx
            .grug_app
            .query_app(grug_types::Json::from_inner(request), height)
            .await?
            .to_json_value()?
            .into_inner())
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

        Ok(status)
    }

    async fn simulate(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Transaction as Json")] tx: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        Ok(app_ctx
            .grug_app
            .simulate(tx)
            .await?
            .to_json_value()?
            .into_inner())
    }
}
