use {
    crate::graphql::types::tendermint::{TxAsyncResponse, TxCommitResponse, TxSyncResponse},
    async_graphql::*,
    base64::{Engine, engine::general_purpose::STANDARD},
    tendermint_rpc::Client,
};

#[derive(Default, Debug)]
pub struct TendermintMutation {}

#[Object]
impl TendermintMutation {
    async fn broadcast_tx_sync(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxSyncResponse, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let http_client = tendermint_rpc::HttpClient::new(app_ctx.tendermint_endpoint.as_str())?;

        let tx_bytes = STANDARD.decode(tx)?;

        Ok(http_client.broadcast_tx_sync(tx_bytes).await?.into())
    }

    async fn broadcast_tx_async(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxAsyncResponse, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let http_client = tendermint_rpc::HttpClient::new(app_ctx.tendermint_endpoint.as_str())?;

        let tx_bytes = STANDARD.decode(tx)?;

        Ok(http_client.broadcast_tx_async(tx_bytes).await?.into())
    }

    async fn broadcast_tx_commit(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(desc = "The base64 encoded transaction to broadcast")] tx: String,
    ) -> Result<TxCommitResponse, Error> {
        let app_ctx = ctx.data::<crate::context::Context>()?;

        let http_client = tendermint_rpc::HttpClient::new(app_ctx.tendermint_endpoint.as_str())?;

        let tx_bytes = STANDARD.decode(tx)?;

        Ok(http_client.broadcast_tx_commit(tx_bytes).await?.into())
    }
}
