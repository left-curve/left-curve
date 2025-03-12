use {
    crate::{
        Addr, Binary, Code, Coins, Config, ContractInfo, Denom, Hash256, JsonDeExt, Query,
        QueryRequest, QueryResponse, StdError,
    },
    grug_math::Uint128,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{collections::BTreeMap, fmt::Debug},
};

pub trait Querier {
    type Error;

    /// Make a query. This is the only method that the context needs to manually
    /// implement. The other methods will be implemented automatically.
    fn query_chain(&self, req: Query) -> Result<QueryResponse, Self::Error>;
}

/// Core querying functionality that builds on top of the base `Querier` trait.
///
/// This trait exists separately from `Querier` because it contains generic
/// methods (e.g. `query_wasm_smart`), which would make the trait
/// non-object-safe. By keeping these methods in a separate trait, we can still
/// use `dyn Querier`. This trait is automatically implemented for any type that
/// implements `Querier`.
pub trait QuerierExt: Querier
where
    Self::Error: From<StdError> + Debug,
{
    fn query_config(&self) -> Result<Config, Self::Error> {
        self.query_chain(Query::config()).map(|res| res.as_config())
    }

    fn query_owner(&self) -> Result<Addr, Self::Error> {
        self.query_config().map(|res| res.owner)
    }

    fn query_bank(&self) -> Result<Addr, Self::Error> {
        self.query_config().map(|res| res.bank)
    }

    fn query_taxman(&self) -> Result<Addr, Self::Error> {
        self.query_config().map(|res| res.taxman)
    }

    fn query_app_config<T>(&self) -> Result<T, Self::Error>
    where
        T: DeserializeOwned,
    {
        self.query_chain(Query::app_config())
            .and_then(|res| res.as_app_config().deserialize_json().map_err(Into::into))
    }

    fn query_balance(&self, address: Addr, denom: Denom) -> Result<Uint128, Self::Error> {
        self.query_chain(Query::balance(address, denom))
            .map(|res| res.as_balance().amount)
    }

    fn query_balances(
        &self,
        address: Addr,
        start_after: Option<Denom>,
        limit: Option<u32>,
    ) -> Result<Coins, Self::Error> {
        self.query_chain(Query::balances(address, start_after, limit))
            .map(|res| res.as_balances())
    }

    fn query_supply(&self, denom: Denom) -> Result<Uint128, Self::Error> {
        self.query_chain(Query::supply(denom))
            .map(|res| res.as_supply().amount)
    }

    fn query_supplies(
        &self,
        start_after: Option<Denom>,
        limit: Option<u32>,
    ) -> Result<Coins, Self::Error> {
        self.query_chain(Query::supplies(start_after, limit))
            .map(|res| res.as_supplies())
    }

    fn query_code(&self, hash: Hash256) -> Result<Code, Self::Error> {
        self.query_chain(Query::code(hash)).map(|res| res.as_code())
    }

    fn query_codes(
        &self,
        start_after: Option<Hash256>,
        limit: Option<u32>,
    ) -> Result<BTreeMap<Hash256, Code>, Self::Error> {
        self.query_chain(Query::codes(start_after, limit))
            .map(|res| res.as_codes())
    }

    fn query_contract(&self, address: Addr) -> Result<ContractInfo, Self::Error> {
        self.query_chain(Query::contract(address))
            .map(|res| res.as_contract())
    }

    fn query_contracts(
        &self,
        start_after: Option<Addr>,
        limit: Option<u32>,
    ) -> Result<BTreeMap<Addr, ContractInfo>, Self::Error> {
        self.query_chain(Query::contracts(start_after, limit))
            .map(|res| res.as_contracts())
    }

    /// Note: In most cases, for querying a single storage path in another
    /// contract, the `StorageQuerier::query_wasm_path` method is preferred.
    ///
    /// The only case where `query_wasm_raw` is preferred is if you just want to
    /// know whether a data exists or not, without needing to deserialize it.
    fn query_wasm_raw<B>(&self, contract: Addr, key: B) -> Result<Option<Binary>, Self::Error>
    where
        B: Into<Binary>,
    {
        self.query_chain(Query::wasm_raw(contract, key))
            .map(|res| res.as_wasm_raw())
    }

    fn query_wasm_smart<R>(&self, contract: Addr, req: R) -> Result<R::Response, Self::Error>
    where
        R: QueryRequest,
        R::Message: Serialize,
        R::Response: DeserializeOwned,
    {
        let msg = R::Message::from(req);

        self.query_chain(Query::wasm_smart(contract, &msg)?)
            .and_then(|res| res.as_wasm_smart().deserialize_json().map_err(Into::into))
    }

    fn query_multi<const N: usize>(
        &self,
        requests: [Query; N],
    ) -> Result<[Result<QueryResponse, Self::Error>; N], Self::Error> {
        self.query_chain(Query::Multi(requests.into())).map(|res| {
            // We trust that the host has properly implemented the multi
            // query method, meaning the number of responses should always
            // match the number of requests.

            // let res = res.as_multi();

            let responses = res
                .as_multi()
                .into_iter()
                .map(|res| res.map_err(|err| StdError::host(err).into()))
                .collect::<Vec<Result<QueryResponse, Self::Error>>>();

            debug_assert_eq!(
                responses.len(),
                N,
                "number of responses ({}) does not match that of requests ({})",
                responses.len(),
                N
            );
            // responses
            responses.try_into().unwrap()
        })
    }
}

impl<Q> QuerierExt for Q
where
    Q: Querier,
    Q::Error: From<StdError> + Debug,
{
}

/// Wraps around a `Querier` to provide some convenience methods.
///
/// We have to do this because `&dyn Querier` itself doesn't implement `Querier`,
/// so given a `&dyn Querier` you aren't able to access the `QuerierExt` methods.
pub struct QuerierWrapper<'a, E = StdError> {
    inner: &'a dyn Querier<Error = E>,
}

impl<E> Querier for QuerierWrapper<'_, E> {
    type Error = E;

    fn query_chain(&self, req: Query) -> Result<QueryResponse, Self::Error> {
        self.inner.query_chain(req)
    }
}

impl<'a, E> QuerierWrapper<'a, E> {
    pub fn new(inner: &'a dyn Querier<Error = E>) -> Self {
        Self { inner }
    }
}
