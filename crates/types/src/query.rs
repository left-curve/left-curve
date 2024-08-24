use {
    crate::{Account, Addr, Binary, BlockInfo, Coin, Coins, Config, Hash256, Json},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
    std::collections::BTreeMap,
};

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
    Balance { address: Addr, denom: String },
    /// Enumerate an account's balances in all denoms.
    /// Returns: `Coins`
    Balances {
        address: Addr,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// A token's total supply.
    /// Returns: `Coin`
    Supply { denom: String },
    /// Enumerate all token's total supplies.
    /// Returns: `Coins`
    Supplies {
        start_after: Option<String>,
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
    /// Metadata of a single account.
    /// Returns: `Account`
    Account { address: Addr },
    /// Enumerate metadata of all accounts.
    /// Returns: `BTreeMap<Addr, Account>`
    Accounts {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InfoResponse {
    pub chain_id: String,
    pub config: Config,
    pub last_finalized_block: BlockInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[grug_macros::downcast_enum]
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
    Account(Account),
    Accounts(BTreeMap<Addr, Account>),
    WasmRaw(Option<Binary>),
    WasmSmart(Json),
    Multi(Vec<QueryResponse>),
}
