use {
    crate::{
        Addr, Binary, Code, Coins, Config, ContractInfo, Denom, Hash256, JsonDeExt, NextUpgrade,
        PastUpgrade, Query, QueryRequest, QueryResponse, QueryStatusResponse, StdError, TxOutcome,
        UnsignedTx,
    },
    async_trait::async_trait,
    dango_math::Uint128,
    serde::{Serialize, de::DeserializeOwned},
    std::collections::BTreeMap,
};

#[async_trait]
pub trait QueryClient: Send + Sync {
    type Error: From<StdError>;
    type Proof;

    async fn query_app(&self, query: Query) -> Result<QueryResponse, Self::Error>;

    async fn simulate(&self, tx: UnsignedTx) -> Result<TxOutcome, Self::Error>;
}

#[async_trait]
pub trait QueryClientExt: QueryClient {
    async fn query_status(&self) -> Result<QueryStatusResponse, Self::Error> {
        self.query_app(Query::status())
            .await
            .map(|res| res.into_status())
    }

    async fn query_config(&self) -> Result<Config, Self::Error> {
        self.query_app(Query::config())
            .await
            .map(|res| res.into_config())
    }

    async fn query_owner(&self) -> Result<Addr, Self::Error> {
        self.query_config().await.map(|res| res.owner)
    }

    async fn query_bank(&self) -> Result<Addr, Self::Error> {
        self.query_config().await.map(|res| res.bank)
    }

    async fn query_app_config<T>(&self) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        self.query_app(Query::app_config())
            .await
            .and_then(|res| res.into_app_config().deserialize_json().map_err(Into::into))
    }

    async fn query_next_upgrade(&self) -> Result<Option<NextUpgrade>, Self::Error> {
        self.query_app(Query::next_upgrade())
            .await
            .map(|res| res.into_next_upgrade())
    }

    async fn query_past_upgrades(
        &self,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> Result<BTreeMap<u64, PastUpgrade>, Self::Error> {
        self.query_app(Query::past_upgrades(start_after, limit))
            .await
            .map(|res| res.into_past_upgrades())
    }

    async fn query_balance(&self, address: Addr, denom: Denom) -> Result<Uint128, Self::Error> {
        self.query_app(Query::balance(address, denom))
            .await
            .map(|res| res.into_balance().amount)
    }

    async fn query_balances(
        &self,
        address: Addr,
        start_after: Option<Denom>,
        limit: Option<u32>,
    ) -> Result<Coins, Self::Error> {
        self.query_app(Query::balances(address, start_after, limit))
            .await
            .map(|res| res.into_balances())
    }

    async fn query_supply(&self, denom: Denom) -> Result<Uint128, Self::Error> {
        self.query_app(Query::supply(denom))
            .await
            .map(|res| res.into_supply().amount)
    }

    async fn query_supplies(
        &self,
        start_after: Option<Denom>,
        limit: Option<u32>,
    ) -> Result<Coins, Self::Error> {
        self.query_app(Query::supplies(start_after, limit))
            .await
            .map(|res| res.into_supplies())
    }

    async fn query_code(&self, hash: Hash256) -> Result<Code, Self::Error> {
        self.query_app(Query::code(hash))
            .await
            .map(|res| res.into_code())
    }

    async fn query_codes(
        &self,
        start_after: Option<Hash256>,
        limit: Option<u32>,
    ) -> Result<BTreeMap<Hash256, Code>, Self::Error> {
        self.query_app(Query::codes(start_after, limit))
            .await
            .map(|res| res.into_codes())
    }

    async fn query_contract(&self, address: Addr) -> Result<ContractInfo, Self::Error> {
        self.query_app(Query::contract(address))
            .await
            .map(|res| res.into_contract())
    }

    async fn query_contracts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
    ) -> Result<BTreeMap<Addr, ContractInfo>, Self::Error> {
        self.query_app(Query::contracts(start_after, limit))
            .await
            .map(|res| res.into_contracts())
    }

    /// Note: In most cases, for querying a single storage path in another
    /// contract, the `StorageQuerier::query_wasm_path` method is preferred.
    ///
    /// The only case where `query_wasm_raw` is preferred is if you just want to
    /// know whether a data exists or not, without needing to deserialize it.
    async fn query_wasm_raw<B>(&self, contract: Addr, key: B) -> Result<Option<Binary>, Self::Error>
    where
        B: Into<Binary> + Send,
    {
        self.query_app(Query::wasm_raw(contract, key))
            .await
            .map(|res| res.into_wasm_raw())
    }

    async fn query_wasm_smart<R>(&self, contract: Addr, req: R) -> Result<R::Response, Self::Error>
    where
        R: QueryRequest + Send,
        R::Message: Serialize + Send,
        R::Response: DeserializeOwned,
    {
        let msg = R::Message::from(req);

        self.query_app(Query::wasm_smart(contract, &msg)?)
            .await
            .and_then(|res| res.into_wasm_smart().deserialize_json().map_err(Into::into))
    }

    async fn query_multi<const N: usize>(
        &self,
        requests: [Query; N],
    ) -> Result<[Result<QueryResponse, Self::Error>; N], Self::Error> {
        self.query_app(Query::Multi(requests.into()))
            .await
            .map(|res| {
                // We trust that the host has properly implemented the multi
                // query method, meaning the number of responses should always
                // match the number of requests.
                let res = res.into_multi();

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
                        .map_err(StdError::Host)
                        .map_err(Into::into)
                })
            })
    }
}

impl<C> QueryClientExt for C where C: QueryClient {}
