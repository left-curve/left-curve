use {
    crate::graphql::types::tendermint::AbciQuery,
    async_graphql::*,
    base64::{engine::general_purpose::STANDARD, Engine},
    tendermint_rpc::Client,
};

#[derive(Default, Debug)]
pub struct TendermintQuery {}

#[Object]
impl TendermintQuery {
    async fn abci_query(
        &self,
        ctx: &async_graphql::Context<'_>,
        path: Option<String>,
        #[graphql(desc = "The base64 encoded data")] data: String,
        height: Option<u64>,
        #[graphql(default = false)] prove: bool,
    ) -> Result<AbciQuery, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let http_client = tendermint_rpc::HttpClient::new(app_ctx.tendermint_endpoint.as_str())?;

        let height: Option<tendermint::block::Height> = match height {
            Some(h) => Some(h.try_into()?),
            None => None,
        };

        let data = STANDARD.decode(data)?;

        Ok(http_client
            .abci_query(path, data, height, prove)
            .await?
            .into())
    }
}
