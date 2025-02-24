use {
    crate::graphql::types::tendermint::{TxAsyncResponse, TxCommitResponse, TxSyncResponse},
    async_graphql::*,
    base64::{engine::general_purpose::STANDARD, Engine},
    std::sync::OnceLock,
    tendermint_rpc::Client,
};

static TENDERMINT_HTTP_CLIENT: OnceLock<tendermint_rpc::HttpClient> = OnceLock::new();

pub fn get_http_client() -> &'static tendermint_rpc::HttpClient {
    TENDERMINT_HTTP_CLIENT.get_or_init(|| {
        tendermint_rpc::HttpClient::new("http://localhost:26657")
            .expect("Can't build a tendermint RPC client")
    })
}

#[derive(Default, Debug)]
pub struct TendermintMutation {}

#[Object]
impl TendermintMutation {
    async fn broadcast_tx_sync(
        &self,
        _ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxSyncResponse, Error> {
        let client = get_http_client();
        let tx_bytes = STANDARD.decode(tx)?;

        Ok(client.broadcast_tx_sync(tx_bytes).await?.into())
    }

    async fn broadcast_tx_async(
        &self,
        _ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxAsyncResponse, Error> {
        let client = get_http_client();
        let tx_bytes = STANDARD.decode(tx)?;

        Ok(client.broadcast_tx_async(tx_bytes).await?.into())
    }

    async fn broadcast_tx_commit(
        &self,
        _ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxCommitResponse, Error> {
        let client = get_http_client();
        let tx_bytes = STANDARD.decode(tx)?;

        Ok(client.broadcast_tx_commit(tx_bytes).await?.into())
    }
}
