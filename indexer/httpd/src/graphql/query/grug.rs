use async_graphql::*;

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
