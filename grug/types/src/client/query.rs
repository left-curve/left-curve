use {
    crate::{
        Addr, Binary, Code, Coins, Config, ContractInfo, Denom, Hash256, JsonDeExt, Query,
        QueryRequest, QueryResponse, StdError, TxOutcome, UnsignedTx,
    },
    async_trait::async_trait,
    grug_math::Uint128,
    serde::{Serialize, de::DeserializeOwned},
    std::collections::BTreeMap,
};

#[async_trait]
pub trait QueryClient: Send + Sync {
    type Error: From<StdError>;
    type Proof: DeserializeOwned;

    async fn query_app(
        &self,
        query: Query,
        height: Option<u64>,
    ) -> Result<QueryResponse, Self::Error>;

    async fn query_store(
        &self,
        key: Binary,
        height: Option<u64>,
        prove: bool,
    ) -> Result<(Option<Binary>, Option<Self::Proof>), Self::Error>;

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error>;
}

#[async_trait]
pub trait QueryClientExt: QueryClient
where
    Self::Error: From<StdError>,
{
    async fn query_config(&self, height: Option<u64>) -> Result<Config, Self::Error> {
        self.query_app(Query::config(), height)
            .await
            .map(|res| res.as_config())
    }

    async fn query_owner(&self, height: Option<u64>) -> Result<Addr, Self::Error> {
        self.query_config(height).await.map(|res| res.owner)
    }

    async fn query_bank(&self, height: Option<u64>) -> Result<Addr, Self::Error> {
        self.query_config(height).await.map(|res| res.bank)
    }

    async fn query_taxman(&self, height: Option<u64>) -> Result<Addr, Self::Error> {
        self.query_config(height).await.map(|res| res.taxman)
    }

    async fn query_app_config<T>(&self, height: Option<u64>) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        self.query_app(Query::app_config(), height)
            .await
            .and_then(|res| res.as_app_config().deserialize_json().map_err(Into::into))
    }

    async fn query_balance(
        &self,
        address: Addr,
        denom: Denom,
        height: Option<u64>,
    ) -> Result<Uint128, Self::Error> {
        self.query_app(Query::balance(address, denom), height)
            .await
            .map(|res| res.as_balance().amount)
    }

    async fn query_balances(
        &self,
        address: Addr,
        start_after: Option<Denom>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<Coins, Self::Error> {
        self.query_app(Query::balances(address, start_after, limit), height)
            .await
            .map(|res| res.as_balances())
    }

    async fn query_supply(
        &self,
        denom: Denom,
        height: Option<u64>,
    ) -> Result<Uint128, Self::Error> {
        self.query_app(Query::supply(denom), height)
            .await
            .map(|res| res.as_supply().amount)
    }

    async fn query_supplies(
        &self,
        start_after: Option<Denom>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<Coins, Self::Error> {
        self.query_app(Query::supplies(start_after, limit), height)
            .await
            .map(|res| res.as_supplies())
    }

    async fn query_code(&self, hash: Hash256, height: Option<u64>) -> Result<Code, Self::Error> {
        self.query_app(Query::code(hash), height)
            .await
            .map(|res| res.as_code())
    }

    async fn query_codes(
        &self,
        start_after: Option<Hash256>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<BTreeMap<Hash256, Code>, Self::Error> {
        self.query_app(Query::codes(start_after, limit), height)
            .await
            .map(|res| res.as_codes())
    }

    async fn query_contract(
        &self,
        address: Addr,
        height: Option<u64>,
    ) -> Result<ContractInfo, Self::Error> {
        self.query_app(Query::contract(address), height)
            .await
            .map(|res| res.as_contract())
    }

    async fn query_contracts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
        height: Option<u64>,
    ) -> Result<BTreeMap<Addr, ContractInfo>, Self::Error> {
        self.query_app(Query::contracts(start_after, limit), height)
            .await
            .map(|res| res.as_contracts())
    }

    /// Note: In most cases, for querying a single storage path in another
    /// contract, the `StorageQuerier::query_wasm_path` method is preferred.
    ///
    /// The only case where `query_wasm_raw` is preferred is if you just want to
    /// know whether a data exists or not, without needing to deserialize it.
    async fn query_wasm_raw<B>(
        &self,
        contract: Addr,
        key: B,
        height: Option<u64>,
    ) -> Result<Option<Binary>, Self::Error>
    where
        B: Into<Binary> + Send,
    {
        self.query_app(Query::wasm_raw(contract, key), height)
            .await
            .map(|res| res.as_wasm_raw())
    }

    async fn query_wasm_smart<R>(
        &self,
        contract: Addr,
        req: R,
        height: Option<u64>,
    ) -> Result<R::Response, Self::Error>
    where
        R: QueryRequest + Send,
        R::Message: Serialize + Send,
        R::Response: DeserializeOwned,
    {
        let msg = R::Message::from(req);

        self.query_app(Query::wasm_smart(contract, &msg)?, height)
            .await
            .and_then(|res| res.as_wasm_smart().deserialize_json().map_err(Into::into))
    }

    async fn query_multi<const N: usize>(
        &self,
        requests: [Query; N],
        height: Option<u64>,
    ) -> Result<[Result<QueryResponse, Self::Error>; N], Self::Error> {
        self.query_app(Query::Multi(requests.into()), height)
            .await
            .map(|res| {
                // We trust that the host has properly implemented the multi
                // query method, meaning the number of responses should always
                // match the number of requests.
                let res = res.as_multi();

                assert_eq!(
                    res.len(),
                    N,
                    "number of responses ({}) does not match that of requests ({})",
                    res.len(),
                    N
                );

                let mut iter = res.into_iter();

                std::array::from_fn(|_| {
                    iter.next()
                    .unwrap() // unwrap is safe because we've checked the length.
                    .map_err(StdError::host)
                    .map_err(Into::into)
                })
            })
    }
}

impl<C> QueryClientExt for C
where
    C: QueryClient,
    C::Error: From<StdError>,
{
}
