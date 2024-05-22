use {
    crate::prompt::print_json_pretty,
    anyhow::ensure,
    clap::{Parser, Subcommand},
    grug_jmt::Proof,
    grug_sdk::Client,
    grug_types::{Addr, Binary, Hash},
    serde::Serialize,
    serde_json::Value,
    std::{fs::File, io::Write, path::PathBuf},
};

#[derive(Parser)]
pub struct QueryCmd {
    /// Tendermint RPC address
    #[arg(long, global = true, default_value = "http://127.0.0.1:26657")]
    node: String,

    /// The block height at which to perform queries [default: last finalized height]
    #[arg(long, global = true)]
    height: Option<u64>,

    #[command(subcommand, next_display_order = None)]
    subcmd: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
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
        /// Whether to request Merkle proof for raw store queries [default: false]
        #[arg(long, global = true, default_value_t = false)]
        prove: bool,
    },
    /// Get transaction by hash
    Tx {
        /// Transaction hash
        hash: String,
    },
    /// Get block by height
    Block {
        /// Block height [default: latest]
        height: Option<u64>,
    },
}

impl QueryCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let client = Client::connect(&self.node)?;
        match self.subcmd {
            SubCmd::Info => print_json_pretty(client.query_info(self.height).await?),
            SubCmd::Balance {
                address,
                denom,
            } => print_json_pretty(client.query_balance(address, denom, self.height).await?),
            SubCmd::Balances {
                address,
                start_after,
                limit,
            } => print_json_pretty(client.query_balances(address, start_after, limit, self.height).await?),
            SubCmd::Supply {
                denom,
            } => print_json_pretty(client.query_supply(denom, self.height).await?),
            SubCmd::Supplies {
                start_after,
                limit,
            } => print_json_pretty(client.query_supplies(start_after, limit, self.height).await?),
            SubCmd::Code {
                hash,
            } => query_code(&client, hash, self.height).await,
            SubCmd::Codes {
                start_after,
                limit,
            } => print_json_pretty(client.query_codes(start_after, limit, self.height).await?),
            SubCmd::Account {
                address,
            } => print_json_pretty(client.query_account(address, self.height).await?),
            SubCmd::Accounts {
                start_after,
                limit,
            } => print_json_pretty(client.query_accounts(start_after, limit, self.height).await?),
            SubCmd::WasmRaw {
                contract,
                key_hex,
            } => query_wasm_raw(&client, contract, key_hex, self.height).await,
            SubCmd::WasmSmart {
                contract,
                msg,
            } => query_wasm_smart(&client, contract, msg, self.height).await,
            SubCmd::Store {
                key,
                prove,
            } => query_store(&client, key, self.height, prove).await,
            SubCmd::Tx {
                hash,
            } => print_json_pretty(client.tx(&hash).await?),
            SubCmd::Block {
                height,
            } => print_json_pretty(client.block_result(height).await?),
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
    print_json_pretty(PrintableQueryStoreResponse {
        key: key_hex,
        value: value.map(hex::encode),
        proof,
    })
}

#[derive(Serialize)]
struct PrintableQueryStoreResponse {
    key:   String,
    value: Option<String>,
    proof: Option<Proof>,
}
