use {
    crate::{Addr, Binary, BlockInfo, Coin, Hash},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Tx {
    pub sender:     Addr,
    pub msgs:       Vec<Message>,
    pub credential: Option<Binary>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum Message {
    StoreCode {
        wasm_byte_code: Binary,
    },
    Instantiate {
        code_hash: Hash,
        msg:       Binary,
        salt:      Binary,
        funds:     Vec<Coin>,
        admin:     Option<Addr>,
    },
    Execute {
        contract: Addr,
        msg:      Binary,
        funds:    Vec<Coin>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum Query {
    /// The chain's global information. Corresponding to the ABCI Info method.
    /// Returns: InfoResponse
    Info {},
    /// Metadata of a single account.
    /// Returns: AccountResponse
    Account {
        address: Addr,
    },
    /// Enumerate metadata of all accounts.
    /// Returns: Vec<AccountResponse>
    Accounts {
        start_after: Option<Addr>,
        limit:       Option<u32>,
    },
    /// A raw key-value pair in a contract's internal state.
    /// Returns: WasmRawResponse
    WasmRaw {
        contract: Addr,
        key:      Binary,
    },
    /// Call the contract's query entry point with the given message.
    /// Returns: WasmSmartResponse
    WasmSmart {
        contract: Addr,
        msg:      Binary,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InfoResponse {
    pub chain_id:             String,
    pub last_finalized_block: BlockInfo,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AccountResponse {
    pub address:   Addr,
    pub code_hash: Hash,
    pub admin:     Option<Addr>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WasmRawResponse {
    pub contract: Addr,
    pub key:      Binary,
    pub value:    Option<Binary>, // None if key doesn't exist
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WasmSmartResponse {
    pub contract: Addr,
    pub data:     Binary,
}
