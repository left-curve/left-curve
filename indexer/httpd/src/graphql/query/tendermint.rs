use {
    crate::graphql::{mutation::tendermint::get_http_client, types::tendermint::AbciQuery},
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
        _ctx: &async_graphql::Context<'_>,
        path: Option<String>,
        #[graphql(desc = "The base64 encoded data")] data: String,
        height: Option<u64>,
        #[graphql(default = false)] prove: bool,
    ) -> Result<AbciQuery, Error> {
        let client = get_http_client();

        let height: Option<tendermint::block::Height> = match height {
            Some(h) => Some(h.try_into()?),
            None => None,
        };

        let data = STANDARD.decode(data)?;

        Ok(client.abci_query(path, data, height, prove).await?.into())
    }
}
