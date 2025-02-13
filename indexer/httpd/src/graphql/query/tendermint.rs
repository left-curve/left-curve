use {
    crate::graphql::{mutation::tendermint::get_http_client, types::tendermint::AbciQuery},
    async_graphql::*,
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
        // data: V,
        height: Option<u64>,
        prove: bool,
    ) -> Result<AbciQuery, Error> {
        let client = get_http_client();
        let data = "".to_string();

        let height: Option<tendermint::block::Height> = match height {
            Some(h) => Some(h.try_into()?),
            None => None,
        };

        Ok(client.abci_query(path, data, height, prove).await?.into())
    }
}
