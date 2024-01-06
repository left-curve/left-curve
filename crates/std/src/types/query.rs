use {
    crate::{Addr, Binary, BlockInfo, Hash},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Query {
    /// The chain's global information. Corresponding to the ABCI Info method.
    /// Returns: InfoResponse
    Info {},
    /// A single Wasm byte code.
    /// Returns: Binary
    Code {
        hash: Hash,
    },
    /// Enumerate metadata of all codes.
    /// Note: to limit the size of return data, we only return the hashes.
    /// To download the actual Wasm byte code, use Query::Code.
    /// Returns: Vec<Hash>
    Codes {
        start_after: Option<Hash>,
        limit:       Option<u32>,
    },
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryResponse {
    Info(InfoResponse),
    Code(Binary),
    Codes(Vec<Hash>),
    Account(AccountResponse),
    Accounts(Vec<AccountResponse>),
    WasmRaw(WasmRawResponse),
    WasmSmart(WasmSmartResponse),
}

// TODO: can we use a macro to implement these?
impl QueryResponse {
    pub fn as_info(self) -> InfoResponse {
        let Self::Info(resp) = self else {
            panic!("QueryResponse is not Info");
        };
        resp
    }

    pub fn as_code(self) -> Binary {
        let Self::Code(wasm_byte_code) = self else {
            panic!("QueryResponse is not Code");
        };
        wasm_byte_code
    }

    pub fn as_codes(self) -> Vec<Hash> {
        let Self::Codes(hashes) = self else {
            panic!("QueryResponse is not Codes");
        };
        hashes
    }

    pub fn as_account(self) -> AccountResponse {
        let Self::Account(resp) = self else {
            panic!("QueryResponse is not Account");
        };
        resp
    }

    pub fn as_accounts(self) -> Vec<AccountResponse> {
        let Self::Accounts(resp) = self else {
            panic!("QueryResponse is not Accounts");
        };
        resp
    }

    pub fn as_wasm_raw(self) -> WasmRawResponse {
        let Self::WasmRaw(resp) = self else {
            panic!("QueryResponse is not WasmRaw");
        };
        resp
    }

    pub fn as_wasm_smart(self) -> WasmSmartResponse {
        let Self::WasmSmart(resp) = self else {
            panic!("QueryResponse is not WasmSmart");
        };
        resp
    }
}
