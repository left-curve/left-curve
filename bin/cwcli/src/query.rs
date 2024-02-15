use {
    crate::prompt::print_json_pretty,
    anyhow::ensure,
    clap::Parser,
    cw_jmt::Proof,
    cw_rs::Client,
    cw_std::{Addr, Binary, Hash},
    serde::Serialize,
    serde_json::Value,
    std::{fs::File, io::Write, path::PathBuf},
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
        key_hex: String,
    },
    /// Call a contract's query entry point
    WasmSmart {
        /// Contract address
        contract: Addr,
        /// JSON-encoded query message
        msg: String,
    },
    /// Query a raw key in the store
    Store {
        /// Key in hex encoding
        key: String,
    },
}

impl QueryCmd {
    pub async fn run(self, rpc_addr: &str, height: Option<u64>, prove: bool) -> anyhow::Result<()> {
        let client = Client::connect(rpc_addr)?;
        match self {
            QueryCmd::Info => print_json_pretty(client.query_info(height).await?),
            QueryCmd::Balance {
                address,
                denom,
            } => print_json_pretty(client.query_balance(address, denom, height).await?),
            QueryCmd::Balances {
                address,
                start_after,
                limit,
            } => print_json_pretty(client.query_balances(address, start_after, limit, height).await?),
            QueryCmd::Supply {
                denom,
            } => print_json_pretty(client.query_supply(denom, height).await?),
            QueryCmd::Supplies {
                start_after,
                limit,
            } => print_json_pretty(client.query_supplies(start_after, limit, height).await?),
            QueryCmd::Code {
                hash,
            } => query_code(&client, hash, height).await,
            QueryCmd::Codes {
                start_after,
                limit,
            } => print_json_pretty(client.query_codes(start_after, limit, height).await?),
            QueryCmd::Account {
                address,
            } => print_json_pretty(client.query_account(address, height).await?),
            QueryCmd::Accounts {
                start_after,
                limit,
            } => print_json_pretty(client.query_accounts(start_after, limit, height).await?),
            QueryCmd::WasmRaw {
                contract,
                key_hex,
            } => query_wasm_raw(&client, contract, key_hex, height).await,
            QueryCmd::WasmSmart {
                contract,
                msg,
            } => query_wasm_smart(&client, contract, msg, height).await,
            QueryCmd::Store {
                key,
            } => query_store(&client, key, height, prove).await,
        }
    }
}

async fn query_code(client: &Client, hash: Hash, height: Option<u64>) -> anyhow::Result<()> {
    // we will be writing the wasm byte code to $(pwd)/${hash}.wasm
    // first check if the file already exists. throw if it does
    let filename = PathBuf::from(format!("{hash}.wasm"));
    ensure!(!filename.exists(), "file `{filename:?}` already exists!");

    // make the query
    let wasm_byte_code = client.query_code(hash, height).await?;

    // write the bytes to the file
    // this creates a new file if not exists, an overwrite if exists
    let mut file = File::create(&filename)?;
    file.write_all(&wasm_byte_code)?;

    println!("Wasm byte code written to {filename:?}");

    Ok(())
}

async fn query_wasm_raw(
    client:   &Client,
    contract: Addr,
    key_hex:  String,
    height:   Option<u64>,
) -> anyhow::Result<()> {
    // we interpret the input raw key as Hex encoded
    let key = Binary::from(hex::decode(&key_hex)?);
    print_json_pretty(client.query_wasm_raw(contract, key, height).await?)
}

async fn query_wasm_smart(
    client:   &Client,
    contract: Addr,
    msg:      String,
    height:   Option<u64>,
) -> anyhow::Result<()> {
    // the input should be a JSON string, e.g. `{"config":{}}`
    let msg: Value = serde_json::from_str(&msg)?;
    print_json_pretty(client.query_wasm_smart::<_, Value>(contract, &msg, height).await?)
}

async fn query_store(
    client:  &Client,
    key_hex: String,
    height:  Option<u64>,
    prove:   bool,
) -> anyhow::Result<()> {
    let key = hex::decode(&key_hex)?;
    let (value, proof) = client.query_store(key, height, prove).await?;
    print_json_pretty(&PrintableQueryStoreResponse {
        key: key_hex,
        value: value.map(|v| hex::encode(v)),
        proof,
    })
}

#[derive(Serialize)]
struct PrintableQueryStoreResponse {
    key:   String,
    value: Option<String>,
    proof: Option<Proof>,
}
