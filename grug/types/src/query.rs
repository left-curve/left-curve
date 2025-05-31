#[cfg(feature = "async-graphql")]
use {
    async_graphql::{
        InputType, InputValueResult, OutputType, Positioned, ServerResult,
        context::ContextSelectionSet, parser::types::Field, registry::Registry,
    },
    std::borrow::Cow,
};
// #[cfg(feature = "async-graphql")]
// use {
//     crate::serializers::JsonDeExt,
//     async_graphql::{Scalar, ScalarType},
//     std::borrow::Cow,
// };
use {
    crate::{
        Addr, Binary, Bound, Code, Coin, Coins, Config, ContractInfo, Denom, GenericResult,
        Hash256, Inner, Json, JsonSerExt, StdResult, extend_one_byte,
    },
    borsh::{BorshDeserialize, BorshSerialize},
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

#[cfg(feature = "async-graphql")]
impl InputType for Query {
    type RawValueType = Self;

    fn type_name() -> Cow<'static, str> {
        "Query".into()
    }

    fn create_type_info(_registry: &mut Registry) -> String {
        "Query".to_string()
    }

    fn parse(value: Option<async_graphql::Value>) -> InputValueResult<Self> {
        let value = value.ok_or_else(|| {
            async_graphql::InputValueError::expected_type(async_graphql::Value::Null)
        })?;

        let json_str =
            serde_json::to_string(&value).map_err(async_graphql::InputValueError::custom)?;

        serde_json::from_str(&json_str).map_err(async_graphql::InputValueError::custom)
    }

    fn to_value(&self) -> async_graphql::Value {
        let json_str = serde_json::to_string(self).unwrap();
        serde_json::from_str(&json_str).unwrap()
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        Some(self)
    }
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
    Multi(Vec<GenericResult<QueryResponse>>),
}

// #[cfg(feature = "async-graphql")]
// #[Scalar]
// impl ScalarType for QueryResponse {
//     fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
//         match value.into_json() {
//             Ok(json_value) => Json::from_inner(json_value)
//                 .deserialize_json()
//                 .map_err(|err| {
//                     async_graphql::InputValueError::custom(format!("Failed to parse Tx: {}", err))
//                 }),
//             Err(_) => Err(async_graphql::InputValueError::expected_type(
//                 async_graphql::Value::Null,
//             )),
//         }
//     }

//     fn to_value(&self) -> async_graphql::Value {
//         match self.to_json_value() {
//             Ok(json_value) => async_graphql::Value::Object(
//                 serde_json::from_value(json_value.into_inner())
//                     .expect("Failed to convert Json to Value"),
//             ),
//             Err(_) => async_graphql::Value::Null,
//         }
//     }
// }

#[cfg(feature = "async-graphql")]
impl OutputType for QueryResponse {
    fn type_name() -> Cow<'static, str> {
        "QueryResponse".into()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        <async_graphql::types::Json<serde_json::Value> as OutputType>::create_type_info(registry)
    }

    async fn resolve(
        &self,
        ctx: &ContextSelectionSet<'_>,
        field: &Positioned<Field>,
    ) -> ServerResult<async_graphql::Value> {
        async_graphql::types::Json(self.clone())
            .resolve(ctx, field)
            .await
    }
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
        Multi      => Vec<GenericResult<QueryResponse>>,
    }
}
