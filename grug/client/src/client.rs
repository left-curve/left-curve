use {
    crate::HttpClient,
    async_trait::async_trait,
    grug_types::{
        Binary, BroadcastClient, BroadcastTxOutcome, Defined, MaybeDefined, Proof, Query,
        QueryClient, QueryResponse, Tx, TxOutcome, Undefined, UnsignedTx, WithChainId,
    },
    std::ops::Deref,
};

pub struct Client<C, ID = Undefined<String>>
where
    ID: MaybeDefined<String>,
{
    inner: C,
    chain_id: ID,
}

impl Client<HttpClient, Undefined<String>> {
    pub fn new(endpoint: &str) -> Client<HttpClient, Undefined<String>> {
        Self {
            inner: HttpClient::new(endpoint),
            chain_id: Undefined::new(),
        }
    }

    pub fn from_inner<C>(inner: C) -> Client<C, Undefined<String>> {
        Client {
            inner,
            chain_id: Undefined::new(),
        }
    }
}

impl<C> Client<C, Undefined<String>> {
    pub fn enable_broadcasting<CI>(self, chain_id: CI) -> Client<C, Defined<String>>
    where
        CI: Into<String>,
    {
        Client {
            inner: self.inner,
            chain_id: Defined::new(chain_id.into()),
        }
    }
}

impl<C, ID> Deref for Client<C, ID>
where
    ID: MaybeDefined<String>,
{
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[async_trait]
impl<C, ID> QueryClient for Client<C, ID>
where
    C: QueryClient,
    ID: MaybeDefined<String> + Send + Sync,
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
impl<C> BroadcastClient for Client<C, Defined<String>>
where
    C: BroadcastClient + Send + Sync,
{
    type Error = <C as BroadcastClient>::Error;

    async fn broadcast_tx(&self, tx: Tx) -> Result<BroadcastTxOutcome, Self::Error> {
        self.inner.broadcast_tx(tx).await
    }
}

impl<C> WithChainId for Client<C, Defined<String>> {
    fn chain_id(&self) -> &str {
        self.chain_id.inner()
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
