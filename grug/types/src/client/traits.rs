use {
    super::{BroadcastTxOutcome, SearchTxOutcome},
    crate::{
        Block, BlockOutcome, Hash256, HexBinary, Proof, Query, QueryResponse, StdError, Tx,
        TxOutcome, UnsignedTx,
    },
    async_trait::async_trait,
};

#[async_trait]
pub trait QueryAppClient: Send + Sync
where
    Self::Error: From<StdError>,
{
    type Error;

    async fn query_chain(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error>;

    async fn query_store(
        &self,
        key: HexBinary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Vec<u8>>, Option<Proof>), Self::Error>;

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error>;
}

#[async_trait]
pub trait BlockClient {
    type Error;

    async fn query_block(&self, height: Option<u64>) -> Result<Block, Self::Error>;

    async fn query_block_result(&self, height: Option<u64>) -> Result<BlockOutcome, Self::Error>;
}

#[async_trait]
pub trait BroadcastClient {
    type Error;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error>;
}

#[async_trait]
pub trait SearchTxClient {
    type Error;

    async fn search_tx(&self, hash: Hash256) -> Result<SearchTxOutcome, Self::Error>;
}

pub trait WithChainId {
    fn chain_id(&self) -> &str;
}
