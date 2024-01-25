use {
    clap::Parser,
    cw_std::{Addr, Hash},
};

#[derive(Parser)]
pub enum QueryCmd {
    /// Query the chain's global information
    Info,
    /// Query an account's balance in a single denom
    Balance {
        /// Account address
        address: Addr,
        /// Token denomination
        denom: String,
    },
    /// Enumerate an account's balances in all denoms
    Balances {
        /// Account address
        address: Addr,
        /// Start after this token denomination
        start_after: Option<String>,
        /// Maximum number of items to display
        limit: Option<u32>,
    },
    /// Query a token's total supply
    Supply {
        /// Token denomination
        denom: String,
    },
    /// Enumerate all tokens' total supplies
    Supplies {
        /// Start after this token denomination
        start_after: Option<String>,
        /// Maximum number of items to display
        limit: Option<u32>,
    },
    /// Query a Wasm binary code by hash
    Code {
        hash: Hash,
    },
    /// Enumerate hashes of all Wasm byte codes
    Codes {
        /// Start after this hash
        start_after: Option<Hash>,
        /// Maximum number of items to display
        limit: Option<u32>,
    },
    /// Query metadata of a single account by address
    Account {
        /// Account address
        address: Addr,
    },
    /// Enumerate metadata of all accounts
    Accounts {
        /// Start after this address
        start_after: Option<Addr>,
        /// Maximum number of items to display
        limit: Option<u32>,
    },
    /// Query the raw value in a contract store by raw key
    WasmRaw {
        /// Contract address
        contract: Addr,
        /// The raw key in hex encoding
        key: String,
    },
    /// Call a contract's query entry point
    WasmSmart {
        /// Contract address
        contract: Addr,
        /// JSON-encoded query message
        msg: String,
    },
}

impl QueryCmd {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            QueryCmd::Info => query_info(),
            _ => todo!(),
        }
    }
}

fn query_info() -> anyhow::Result<()> {
    todo!()
}
