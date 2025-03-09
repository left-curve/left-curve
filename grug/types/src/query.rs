use {
    crate::{
        Addr, Binary, Bound, Code, Coin, Coins, Config, ContractInfo, Denom, Hash256, Json,
        JsonSerExt, StdResult, extend_one_byte,
    },
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::Inner,
    paste::paste,
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeMap,
};

/// The default number of items to be returned in enumerative queries, if user
/// doesn't specify a limit.
///
/// Exported here for convenience. Of course, individual contracts can choose
/// their own defaults.
pub const DEFAULT_PAGE_LIMIT: u32 = 30;

// ----------------------------------- trait -----------------------------------

/// Represents a query request to a contract.
///
/// A contract typically exposes multiple query methods, with a `QueryMsg` as an
/// enum with multiple variants. A `QueryRequest` represents one such variant.
pub trait QueryRequest: Sized {
    /// The full query message enum that contains this request.
    type Message: From<Self>;

    /// The response type for this query.
    type Response;
}

// ---------------------------------- request ----------------------------------

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Query {
    /// Query the chain's global configuration.
    Config(QueryConfigRequest),
    /// Query the application-specific configuration.
    AppConfig(QueryAppConfigRequest),
    /// Query an account's balance in a single denom.
    Balance(QueryBalanceRequest),
    /// Enumerate an account's balances in all denoms.
    Balances(QueryBalancesRequest),
    /// Query a token's total supply.
    Supply(QuerySupplyRequest),
    /// Enumerate all token's total supplies.
    Supplies(QuerySuppliesRequest),
    /// Query a single Wasm byte code.
    Code(QueryCodeRequest),
    /// Enumerate all Wasm byte codes.
    Codes(QueryCodesRequest),
    /// Query the metadata of a single contract.
    Contract(QueryContractRequest),
    /// Enumerate metadata of all contracts.
    Contracts(QueryContractsRequest),
    /// Query a raw key-value pair in a contract's internal state.
    WasmRaw(QueryWasmRawRequest),
    /// Enumerate key-value pairs in a contract's internal state.
    WasmScan(QueryWasmScanRequest),
    /// Call the contract's query entry point with the given message.
    WasmSmart(QueryWasmSmartRequest),
    /// Perform multiple queries at once.
    Multi(Vec<Query>),
}

impl Query {
    pub fn config() -> Self {
        QueryConfigRequest {}.into()
    }

    pub fn app_config() -> Self {
        QueryAppConfigRequest {}.into()
    }

    pub fn balance(address: Addr, denom: Denom) -> Self {
        QueryBalanceRequest { address, denom }.into()
    }

    pub fn balances(address: Addr, start_after: Option<Denom>, limit: Option<u32>) -> Self {
        QueryBalancesRequest {
            address,
            start_after,
            limit,
        }
        .into()
    }

    pub fn supply(denom: Denom) -> Self {
        QuerySupplyRequest { denom }.into()
    }

    pub fn supplies(start_after: Option<Denom>, limit: Option<u32>) -> Self {
        QuerySuppliesRequest { start_after, limit }.into()
    }

    pub fn code(hash: Hash256) -> Self {
        QueryCodeRequest { hash }.into()
    }

    pub fn codes(start_after: Option<Hash256>, limit: Option<u32>) -> Self {
        QueryCodesRequest { start_after, limit }.into()
    }

    pub fn contract(address: Addr) -> Self {
        QueryContractRequest { address }.into()
    }

    pub fn contracts(start_after: Option<Addr>, limit: Option<u32>) -> Self {
        QueryContractsRequest { start_after, limit }.into()
    }

    pub fn wasm_raw<B>(contract: Addr, key: B) -> Self
    where
        B: Into<Binary>,
    {
        QueryWasmRawRequest {
            contract,
            key: key.into(),
        }
        .into()
    }

    pub fn wasm_scan(
        contract: Addr,
        min: Option<Bound<Binary>>,
        max: Option<Bound<Binary>>,
        limit: Option<u32>,
    ) -> Self {
        let min = min.map(|min| match min {
            Bound::Inclusive(min) => min,
            Bound::Exclusive(min) => extend_one_byte(min.into_inner()).into(),
        });

        let max = max.map(|max| match max {
            Bound::Exclusive(max) => max,
            Bound::Inclusive(max) => extend_one_byte(max.into_inner()).into(),
        });

        QueryWasmScanRequest {
            contract,
            min,
            max,
            limit,
        }
        .into()
    }

    pub fn wasm_smart<M>(contract: Addr, msg: &M) -> StdResult<Self>
    where
        M: Serialize,
    {
        Ok(QueryWasmSmartRequest {
            contract,
            msg: msg.to_json_value()?,
        }
        .into())
    }

    pub fn multi<Q, I>(queries: I) -> Self
    where
        Q: Into<Query>,
        I: IntoIterator<Item = Q>,
    {
        Query::Multi(queries.into_iter().map(|req| req.into()).collect())
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryConfigRequest {}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryAppConfigRequest {}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryAppConfigsRequest {
    pub start_after: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryBalanceRequest {
    pub address: Addr,
    pub denom: Denom,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryBalancesRequest {
    pub address: Addr,
    pub start_after: Option<Denom>,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QuerySupplyRequest {
    pub denom: Denom,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QuerySuppliesRequest {
    pub start_after: Option<Denom>,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryCodeRequest {
    pub hash: Hash256,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryCodesRequest {
    pub start_after: Option<Hash256>,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryContractRequest {
    pub address: Addr,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryContractsRequest {
    pub start_after: Option<Addr>,
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryWasmRawRequest {
    pub contract: Addr,
    pub key: Binary,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryWasmScanRequest {
    pub contract: Addr,
    pub min: Option<Binary>, // inclusive
    pub max: Option<Binary>, // exclusive
    pub limit: Option<u32>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct QueryWasmSmartRequest {
    pub contract: Addr,
    pub msg: Json,
}

macro_rules! impl_into_query {
    ($variant:ident => $req:ty => $res:ty) => {
        impl From<$req> for Query {
            #[inline]
            fn from(req: $req) -> Self {
                Query::$variant(req)
            }
        }
    };
    ($($variant:ident => $req:ty => $resp:ty),+ $(,)?) => {
        $(
            impl_into_query!($variant => $req => $resp);
        )+
    };
}

impl_into_query! {
    Config     => QueryConfigRequest     => Config,
    AppConfig  => QueryAppConfigRequest  => Json,
    Balance    => QueryBalanceRequest    => Coin,
    Balances   => QueryBalancesRequest   => Coins,
    Supply     => QuerySupplyRequest     => Coin,
    Supplies   => QuerySuppliesRequest   => Coins,
    Code       => QueryCodeRequest       => Code,
    Codes      => QueryCodesRequest      => BTreeMap<Hash256, Code>,
    Contract   => QueryContractRequest   => ContractInfo,
    Contracts  => QueryContractsRequest  => BTreeMap<Addr, ContractInfo>,
    WasmRaw    => QueryWasmRawRequest    => Option<Binary>,
    WasmScan   => QueryWasmScanRequest   => BTreeMap<Binary, Binary>,
    WasmSmart  => QueryWasmSmartRequest  => Json,
    Multi      => Vec<Query>             => Vec<QueryResponse>,
}

// --------------------------------- response ----------------------------------

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryResponse {
    Config(Config),
    AppConfig(Json),
    Balance(Coin),
    Balances(Coins),
    Supply(Coin),
    Supplies(Coins),
    Code(Code),
    Codes(BTreeMap<Hash256, Code>),
    Contract(ContractInfo),
    Contracts(BTreeMap<Addr, ContractInfo>),
    WasmRaw(Option<Binary>),
    WasmScan(BTreeMap<Binary, Binary>),
    WasmSmart(Json),
    Multi(Vec<QueryResponse>),
}

macro_rules! generate_downcast {
    ($id:ident => $ret:ty) => {
        paste! {
            pub fn [<as_$id:snake>](self) -> $ret {
                match self {
                    QueryResponse::$id(value) => value,
                    _ => panic!("QueryResponse is not {}", stringify!($id)),
                }
            }
        }
    };
    ($($id:ident => $ret:ty),+ $(,)?) => {
        $(
            generate_downcast!($id => $ret);
        )+
    };
}

impl QueryResponse {
    generate_downcast! {
        Config     => Config,
        AppConfig  => Json,
        Balance    => Coin,
        Balances   => Coins,
        Supply     => Coin,
        Supplies   => Coins,
        Code       => Code,
        Codes      => BTreeMap<Hash256, Code>,
        Contract   => ContractInfo,
        Contracts  => BTreeMap<Addr, ContractInfo>,
        WasmRaw    => Option<Binary>,
        WasmScan   => BTreeMap<Binary, Binary>,
        WasmSmart  => Json,
        Multi      => Vec<QueryResponse>,
    }
}
