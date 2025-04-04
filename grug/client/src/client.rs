use {
    crate::HttpClient,
    async_trait::async_trait,
    grug_types::{
        Binary, BroadcastClient, BroadcastTxOutcome, Proof, Query, QueryClient, QueryResponse, Tx,
        TxOutcome, UnsignedTx,
    },
    std::ops::Deref,
};

pub struct Client<C> {
    inner: C,
}

impl Client<HttpClient> {
    pub fn new(endpoint: &str) -> Client<HttpClient> {
        Self {
            inner: HttpClient::new(endpoint),
        }
    }

    pub fn from_inner<C>(inner: C) -> Client<C> {
        Client { inner }
    }
}

impl<C> Deref for Client<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[async_trait]
impl<C> QueryClient for Client<C>
where
    C: QueryClient,
{
    type Error = C::Error;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error> {
        self.inner.query_app(query, height).await
    }

    async fn query_store(
        &self,
        key: Binary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Binary>, Option<Proof>), Self::Error> {
        self.inner.query_store(key, height, prove).await
    }

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error> {
        self.inner.simulate(tx).await
    }
}

#[async_trait]
impl<C> BroadcastClient for Client<C>
where
    C: BroadcastClient + Send + Sync,
{
    type Error = <C as BroadcastClient>::Error;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        self.inner.broadcast_tx(tx).await
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug_types::QueryClientExt};

    #[tokio::test]
    async fn graphql_client() {
        let client = Client::new("https://devnet-graphql.dango.exchange");

        let response = client.query_config(None).await.unwrap();
        println!("{:?}", response);
    }
}
