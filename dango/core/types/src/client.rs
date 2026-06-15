mod block;
mod broadcast;
mod options;
mod query;
mod search_tx;

pub use {block::*, broadcast::*, options::*, query::*, search_tx::*};

use {
    crate::{
        Binary, Block, BlockOutcome, BroadcastTxOutcome, Hash256, Proof, Query, QueryResponse,
        SearchTxOutcome, StdError, Tx, TxOutcome, UnsignedTx,
    },
    async_trait::async_trait,
    std::sync::Arc,
};

pub trait Client<E, P>:
    BroadcastClient<Error = E>
    + QueryClient<Error = E, Proof = P>
    + SearchTxClient<Error = E>
    + BlockClient<Error = E>
{
}

impl<T, E, P> Client<E, P> for T where
    T: BroadcastClient<Error = E>
        + QueryClient<Error = E, Proof = P>
        + SearchTxClient<Error = E>
        + BlockClient<Error = E>
{
}

pub struct ClientWrapper<E, P = Proof> {
    pub client: Arc<dyn Client<E, P>>,
}

impl<E, P> Clone for ClientWrapper<E, P> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl<E, P> ClientWrapper<E, P> {
    pub fn new(client: Arc<dyn Client<E, P>>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl<E, P> QueryClient for ClientWrapper<E, P>
where
    E: From<StdError>,
{
    type Error = E;
    type Proof = P;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error> {
        self.client.query_app(query, height).await
    }

    async fn query_store(
        &self,
        key: Binary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Binary>, Option<Self::Proof>), Self::Error> {
        self.client.query_store(key, height, prove).await
    }

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error> {
        self.client.simulate(tx).await
    }
}

#[async_trait]
impl<E, P> BroadcastClient for ClientWrapper<E, P> {
    type Error = E;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        self.client.broadcast_tx(tx).await
    }
}

#[async_trait]
impl<E, P> BlockClient for ClientWrapper<E, P> {
    type Error = E;

    async fn query_block(&self, height: Option<u64>) -> Result<Block, Self::Error> {
        self.client.query_block(height).await
    }

    async fn query_block_outcome(&self, height: Option<u64>) -> Result<BlockOutcome, Self::Error> {
        self.client.query_block_outcome(height).await
    }
}

#[async_trait]
impl<E, P> SearchTxClient for ClientWrapper<E, P> {
    type Error = E;

    async fn search_tx(&self, hash: Hash256) -> Result<SearchTxOutcome, Self::Error> {
        self.client.search_tx(hash).await
    }
}
