use {
    async_trait::async_trait,
    grug_types::{BroadcastClient, BroadcastTxOutcome, JsonSerExt, Tx},
    tendermint_rpc::{Client, HttpClient},
};

/// Thin wrapper around [`tendermint_rpc::HttpClient`] that implements
/// [`BroadcastClient`] so it can be plugged into the indexer httpd as a
/// consensus client, forwarding the GraphQL `broadcastTxSync` mutation to
/// a real CometBFT RPC endpoint.
#[derive(Debug, Clone)]
pub struct TendermintRpcClient {
    inner: HttpClient,
}

impl TendermintRpcClient {
    pub fn new(endpoint: &str) -> anyhow::Result<Self> {
        Ok(Self {
            inner: HttpClient::new(endpoint)?,
        })
    }
}

#[async_trait]
impl BroadcastClient for TendermintRpcClient {
    type Error = anyhow::Error;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        let response = self.inner.broadcast_tx_sync(tx.to_json_vec()?).await?;
        Ok(BroadcastTxOutcome::from_tm_broadcast_response(response)?)
    }
}
