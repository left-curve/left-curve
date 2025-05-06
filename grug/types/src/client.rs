mod block;
mod broadcast;
mod options;
mod query;
mod search_tx;

pub use {block::*, broadcast::*, options::*, query::*, search_tx::*};

use {
    crate::{
        Binary, BroadcastTxOutcome, Proof, Query, QueryResponse, StdError, Tx, TxOutcome,
        UnsignedTx,
    },
    async_trait::async_trait,
    std::{ops::Deref, sync::Arc},
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

#[derive(Clone)]
pub struct ClientWrapper<E = anyhow::Error, P = Proof> {
    pub client: Arc<dyn Client<E, P>>,
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

impl<E, P> Deref for ClientWrapper<E, P> {
    type Target = dyn Client<E, P>;

    fn deref(&self) -> &Self::Target {
        self.client.as_ref()
    }
}
