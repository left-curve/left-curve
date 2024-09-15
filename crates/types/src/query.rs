use {
    crate::{
        Addr, Binary, BlockInfo, Coin, Coins, Config, ContractInfo, Denom, Hash256, Json,
        JsonSerExt, StdResult,
    },
    paste::paste,
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeMap,
};

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
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Query {
    /// The chain's global information. Corresponding to the ABCI Info method.
    /// Returns: `InfoResponse`
    Info {},
    /// A single application-specific configuration.
    /// Returns: `Json`
    AppConfig { key: String },
    /// Enumerate all application-specific configurations.
    /// Returns: `BTreeMap<String, Json>`
    AppConfigs {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// An account's balance in a single denom.
    /// Returns: `Coin`
    Balance { address: Addr, denom: Denom },
    /// Enumerate an account's balances in all denoms.
    /// Returns: `Coins`
    Balances {
        address: Addr,
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// A token's total supply.
    /// Returns: `Coin`
    Supply { denom: Denom },
    /// Enumerate all token's total supplies.
    /// Returns: `Coins`
    Supplies {
        start_after: Option<Denom>,
        limit: Option<u32>,
    },
    /// A single Wasm byte code.
    /// Returns: `Binary`
    Code { hash: Hash256 },
    /// Enumerate all Wasm byte codes.
    ///
    /// Returns: `BTreeMap<Hash256, Binary>`
    Codes {
        start_after: Option<Hash256>,
        limit: Option<u32>,
    },
    /// Metadata of a single contract.
    /// Returns: `ContractInfo`
    Contract { address: Addr },
    /// Enumerate metadata of all contracts.
    /// Returns: `BTreeMap<Addr, ContractInfo>`
    Contracts {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    /// A raw key-value pair in a contract's internal state.
    /// Returns: `Option<Binary>`
    WasmRaw { contract: Addr, key: Binary },
    /// Call the contract's query entry point with the given message.
    /// Returns: `Json`
    WasmSmart { contract: Addr, msg: Json },
    /// Perform multiple queries at once.
    /// Returns: `Vec<QueryResponse>`.
    Multi(Vec<Query>),
}

impl Query {
    pub fn wasm_raw<B>(contract: Addr, key: B) -> Self
    where
        B: Into<Binary>,
    {
        Query::WasmRaw {
            contract,
            key: key.into(),
        }
    }

    pub fn wasm_smart<M>(contract: Addr, msg: &M) -> StdResult<Self>
    where
        M: Serialize,
    {
        Ok(Query::WasmSmart {
            contract,
            msg: msg.to_json_value()?,
        })
    }

    pub fn multi<I>(queries: I) -> Self
    where
        I: IntoIterator<Item = Query>,
    {
        Query::Multi(queries.into_iter().collect())
    }
}

// --------------------------------- response ----------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InfoResponse {
    pub chain_id: String,
    pub config: Config,
    pub last_finalized_block: BlockInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryResponse {
    Info(InfoResponse),
    AppConfig(Json),
    AppConfigs(BTreeMap<String, Json>),
    Balance(Coin),
    Balances(Coins),
    Supply(Coin),
    Supplies(Coins),
    Code(Binary),
    Codes(BTreeMap<Hash256, Binary>),
    Contract(ContractInfo),
    Contracts(BTreeMap<Addr, ContractInfo>),
    WasmRaw(Option<Binary>),
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
        Info       => InfoResponse,
        AppConfig  => Json,
        AppConfigs => BTreeMap<String, Json>,
        Balance    => Coin,
        Balances   => Coins,
        Supply     => Coin,
        Supplies   => Coins,
        Code       => Binary,
        Codes      => BTreeMap<Hash256, Binary>,
        Contract   => ContractInfo,
        Contracts  => BTreeMap<Addr, ContractInfo>,
        WasmRaw    => Option<Binary>,
        WasmSmart  => Json,
        Multi      => Vec<QueryResponse>,
    }
}
