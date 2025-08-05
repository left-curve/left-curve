use {
    crate::{
        Addr, Binary, Code, Coins, Config, ContractInfo, Denom, Hash256, JsonDeExt, Query,
        QueryRequest, QueryResponse, QueryStatusResponse, StdError, StdResult,
    },
    grug_math::Uint128,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::collections::BTreeMap,
};

pub trait Querier {
    /// Make a query. This is the only method that the context needs to manually
    /// implement. The other methods will be implemented automatically.
    fn query_chain(&self, req: Query) -> StdResult<QueryResponse>;
}

/// Core querying functionality that builds on top of the base `Querier` trait.
///
/// This trait exists separately from `Querier` because it contains generic
/// methods (e.g. `query_wasm_smart`), which would make the trait
/// non-object-safe. By keeping these methods in a separate trait, we can still
/// use `dyn Querier`. This trait is automatically implemented for any type that
/// implements `Querier`.
pub trait QuerierExt: Querier {
    fn query_status(&self) -> StdResult<QueryStatusResponse> {
        self.query_chain(Query::status()).map(|res| res.as_status())
    }

    fn query_config(&self) -> StdResult<Config> {
        self.query_chain(Query::config()).map(|res| res.as_config())
    }

    fn query_owner(&self) -> StdResult<Addr> {
        self.query_config().map(|res| res.owner)
    }

    fn query_bank(&self) -> StdResult<Addr> {
        self.query_config().map(|res| res.bank)
    }

    fn query_taxman(&self) -> StdResult<Addr> {
        self.query_config().map(|res| res.taxman)
    }

    fn query_bank_and_taxman(&self) -> StdResult<(Addr, Addr)> {
        self.query_config().map(|res| (res.bank, res.taxman))
    }

    fn query_app_config<T>(&self) -> StdResult<T>
    where
        T: DeserializeOwned,
    {
        self.query_chain(Query::app_config())
            .and_then(|res| res.as_app_config().deserialize_json())
    }

    fn query_balance(&self, address: Addr, denom: Denom) -> StdResult<Uint128> {
        self.query_chain(Query::balance(address, denom))
            .map(|res| res.as_balance().amount)
    }

    fn query_balances(
        &self,
        address: Addr,
        start_after: Option<Denom>,
        limit: Option<u32>,
    ) -> StdResult<Coins> {
        self.query_chain(Query::balances(address, start_after, limit))
            .map(|res| res.as_balances())
    }

    fn query_supply(&self, denom: Denom) -> StdResult<Uint128> {
        self.query_chain(Query::supply(denom))
            .map(|res| res.as_supply().amount)
    }

    fn query_supplies(&self, start_after: Option<Denom>, limit: Option<u32>) -> StdResult<Coins> {
        self.query_chain(Query::supplies(start_after, limit))
            .map(|res| res.as_supplies())
    }

    fn query_code(&self, hash: Hash256) -> StdResult<Code> {
        self.query_chain(Query::code(hash)).map(|res| res.as_code())
    }

    fn query_codes(
        &self,
        start_after: Option<Hash256>,
        limit: Option<u32>,
    ) -> StdResult<BTreeMap<Hash256, Code>> {
        self.query_chain(Query::codes(start_after, limit))
            .map(|res| res.as_codes())
    }

    fn query_contract(&self, address: Addr) -> StdResult<ContractInfo> {
        self.query_chain(Query::contract(address))
            .map(|res| res.as_contract())
    }

    fn query_contracts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
    ) -> StdResult<BTreeMap<Addr, ContractInfo>> {
        self.query_chain(Query::contracts(start_after, limit))
            .map(|res| res.as_contracts())
    }

    /// Note: In most cases, for querying a single storage path in another
    /// contract, the `StorageQuerier::query_wasm_path` method is preferred.
    ///
    /// The only case where `query_wasm_raw` is preferred is if you just want to
    /// know whether a data exists or not, without needing to deserialize it.
    fn query_wasm_raw<B>(&self, contract: Addr, key: B) -> StdResult<Option<Binary>>
    where
        B: Into<Binary>,
    {
        self.query_chain(Query::wasm_raw(contract, key))
            .map(|res| res.as_wasm_raw())
    }

    fn query_wasm_smart<R>(&self, contract: Addr, req: R) -> StdResult<R::Response>
    where
        R: QueryRequest,
        R::Message: Serialize,
        R::Response: DeserializeOwned,
    {
        let msg = R::Message::from(req);

        self.query_chain(Query::wasm_smart(contract, &msg)?)
            .and_then(|res| res.as_wasm_smart().deserialize_json())
    }

    fn query_multi<const N: usize>(
        &self,
        requests: [Query; N],
    ) -> StdResult<[StdResult<QueryResponse>; N]> {
        self.query_chain(Query::Multi(requests.into())).map(|res| {
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
            })
        })
    }
}

impl<Q> QuerierExt for Q where Q: Querier {}

/// Wraps around a `Querier` to provide some convenience methods.
///
/// We have to do this because `&dyn Querier` itself doesn't implement `Querier`,
/// so given a `&dyn Querier` you aren't able to access the `QuerierExt` methods.
#[derive(Clone, Copy)]
pub struct QuerierWrapper<'a> {
    inner: &'a dyn Querier,
}

impl Querier for QuerierWrapper<'_> {
    fn query_chain(&self, req: Query) -> StdResult<QueryResponse> {
        self.inner.query_chain(req)
    }
}

impl<'a> QuerierWrapper<'a> {
    pub fn new(inner: &'a dyn Querier) -> Self {
        Self { inner }
    }
}
