use {super::super::types::status::Status, async_graphql::*};

#[derive(Default, Debug)]
pub struct GrugQuery {}

#[Object]
impl GrugQuery {
    async fn query_app(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Request as JSON string")] request: String,
        height: u64,
        #[graphql(default = false)] prove: bool,
    ) -> Result<String, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        Ok(app_ctx.grug_app.query_app(request, height, prove)?)
    }

    async fn query_status(&self, ctx: &async_graphql::Context<'_>) -> Result<Status, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let status = Status {
            block: app_ctx.grug_app.last_block()?.into(),
            chain_id: app_ctx.grug_app.chain_id()?,
        };

        Ok(status)
    }

    async fn simulate(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "Transaction as Json string")] tx: String,
        height: u64,
        #[graphql(default = false)] prove: bool,
    ) -> Result<String, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        Ok(app_ctx.grug_app.simulate(tx, height, prove)?)
    }
}
