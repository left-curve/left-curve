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
    #[arg(long, default_value = "http://127.0.0.1:26657")]
    node: String,

    /// The block height at which to perform queries [default: last finalized height]
    #[arg(long)]
    height: Option<u64>,

    #[command(subcommand)]
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
    Code { hash: Hash },
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
        key_hex: String,
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
            SubCmd::Info => {
                let res = client.query_info(self.height).await?;
                print_json_pretty(res)
            },
            SubCmd::Balance { address, denom } => {
                let res = client.query_balance(address, denom, self.height).await?;
                print_json_pretty(res)
            },
            SubCmd::Balances {
                address,
                start_after,
                limit,
            } => {
                let res = client
                    .query_balances(address, start_after, limit, self.height)
                    .await?;
                print_json_pretty(res)
            },
            SubCmd::Supply { denom } => {
                let res = client.query_supply(denom, self.height).await?;
                print_json_pretty(res)
            },
            SubCmd::Supplies { start_after, limit } => {
                let res = client
                    .query_supplies(start_after, limit, self.height)
                    .await?;
                print_json_pretty(res)
            },
            SubCmd::Code { hash } => {
                // we will be writing the wasm byte code to $(pwd)/${hash}.wasm
                // first check if the file already exists. throw if it does
                let filename = PathBuf::from(format!("{hash}.wasm"));
                ensure!(!filename.exists(), "file `{filename:?}` already exists!");

                // make the query
                let wasm_byte_code = client.query_code(hash, self.height).await?;

                // write the bytes to the file
                // this creates a new file if not exists, an overwrite if exists
                let mut file = File::create(&filename)?;
                file.write_all(&wasm_byte_code)?;

                println!("Wasm byte code written to {filename:?}");

                Ok(())
            },
            SubCmd::Codes { start_after, limit } => {
                let res = client.query_codes(start_after, limit, self.height).await?;
                print_json_pretty(res)
            },
            SubCmd::Account { address } => {
                let res = client.query_account(address, self.height).await?;
                print_json_pretty(res)
            },
            SubCmd::Accounts { start_after, limit } => {
                let res = client
                    .query_accounts(start_after, limit, self.height)
                    .await?;
                print_json_pretty(res)
            },
            SubCmd::WasmRaw { contract, key_hex } => {
                // we interpret the input raw key as Hex encoded
                let key = Binary::from(hex::decode(&key_hex)?);
                let res = client.query_wasm_raw(contract, key, self.height).await?;
                print_json_pretty(res)
            },
            SubCmd::WasmSmart { contract, msg } => {
                // the input should be a JSON string, e.g. `{"config":{}}`
                let msg: Value = serde_json::from_str(&msg)?;
                let res = client
                    .query_wasm_smart::<_, Value>(contract, &msg, self.height)
                    .await?;
                print_json_pretty(res)
            },
            SubCmd::Store { key_hex, prove } => {
                #[derive(Serialize)]
                struct PrintableQueryStoreResponse {
                    key: String,
                    value: Option<String>,
                    proof: Option<Proof>,
                }

                let key = hex::decode(&key_hex)?;
                let (value, proof) = client.query_store(key, self.height, prove).await?;

                print_json_pretty(PrintableQueryStoreResponse {
                    key: key_hex,
                    value: value.map(hex::encode),
                    proof,
                })
            },
            SubCmd::Tx { hash } => {
                let res = client.tx(&hash).await?;
                print_json_pretty(res)
            },
            SubCmd::Block { height } => {
                let res = client.block_result(height).await?;
                print_json_pretty(res)
            },
        }
    }
}
