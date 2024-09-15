use {
    clap::{Parser, Subcommand},
    colored_json::ToColoredJson,
    grug_client::Client,
    grug_jmt::Proof,
    grug_types::{Addr, Binary, Denom, Hash, Hash256, JsonDeExt, JsonSerExt, Query},
    serde::Serialize,
    std::str::FromStr,
};

#[derive(Parser)]
pub struct QueryCmd {
    /// Tendermint RPC address
    #[arg(long, global = true, default_value = "http://127.0.0.1:26657")]
    node: String,

    /// The block height at which to perform queries [default: last finalized height]
    #[arg(long, global = true)]
    height: Option<u64>,

    #[command(subcommand)]
    subcmd: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
    /// Tendermint node status
    Status,
    /// Get transaction by hash
    Tx {
        /// Transaction hash in hex encoding
        hash: String,
    },
    /// Get block results by height
    Block {
        /// Block height [default: latest]
        height: Option<u64>,
    },
    /// Query the chain's global configuration
    Config,
    /// Query a single application-specific configuration
    AppConfig { key: String },
    /// Enumerate all application-specific configurations
    AppConfigs {
        start_after: Option<String>,
        limit: Option<u32>,
    },
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
    Code { hash: Hash256 },
    /// Enumerate all Wasm byte codes
    Codes {
        /// Start after this hash
        start_after: Option<Hash256>,
        /// Maximum number of items to display
        limit: Option<u32>,
    },
    /// Query metadata of a single contract by address
    Contract {
        /// Account address
        address: Addr,
    },
    /// Enumerate metadata of all contracts
    Contracts {
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
}

impl QueryCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let client = Client::connect(&self.node)?;

        let req = match self.subcmd {
            SubCmd::Status {} => {
                let res = client.query_status().await?;
                return print_json_pretty(res);
            },
            SubCmd::Tx { hash } => {
                // Cast the hex string to uppercase, so that users can use
                // either upper or lowercase on the CLI.
                let hash = Hash::from_str(&hash.to_ascii_uppercase())?;
                let res = client.query_tx(hash).await?;
                return print_json_pretty(res);
            },
            SubCmd::Block { height } => {
                let res = client.query_block_result(height).await?;
                return print_json_pretty(res);
            },
            SubCmd::Config => Query::Config {},
            SubCmd::AppConfig { key } => Query::AppConfig { key },
            SubCmd::AppConfigs { start_after, limit } => Query::AppConfigs { start_after, limit },
            SubCmd::Balance { address, denom } => {
                let denom = Denom::try_from(denom)?;
                Query::Balance { address, denom }
            },
            SubCmd::Balances {
                address,
                start_after,
                limit,
            } => {
                let start_after = start_after.map(Denom::try_from).transpose()?;
                Query::Balances {
                    address,
                    start_after,
                    limit,
                }
            },
            SubCmd::Supply { denom } => {
                let denom = Denom::try_from(denom)?;
                Query::Supply { denom }
            },
            SubCmd::Supplies { start_after, limit } => {
                let start_after = start_after.map(Denom::try_from).transpose()?;
                Query::Supplies { start_after, limit }
            },
            SubCmd::Code { hash } => Query::Code { hash },
            SubCmd::Codes { start_after, limit } => Query::Codes { start_after, limit },
            SubCmd::Contract { address } => Query::Contract { address },
            SubCmd::Contracts { start_after, limit } => Query::Contracts { start_after, limit },
            SubCmd::WasmRaw { contract, key_hex } => {
                // We interpret the input raw key as Hex encoded
                let key = Binary::from(hex::decode(key_hex)?);
                Query::WasmRaw { contract, key }
            },
            SubCmd::WasmSmart { contract, msg } => {
                // The input should be a JSON string, e.g. `{"config":{}}`
                let msg = msg.deserialize_json()?;
                Query::WasmSmart { contract, msg }
            },
            SubCmd::Store { key_hex, prove } => {
                return query_store(&client, key_hex, self.height, prove).await;
            },
        };

        client
            .query_app(&req, self.height)
            .await
            .and_then(print_json_pretty)
    }
}

#[derive(Serialize)]
struct PrintableQueryStoreResponse {
    key: String,
    value: Option<String>,
    proof: Option<Proof>,
}

async fn query_store(
    client: &Client,
    key_hex: String,
    height: Option<u64>,
    prove: bool,
) -> anyhow::Result<()> {
    let key = hex::decode(&key_hex)?;
    let (value, proof) = client.query_store(key, height, prove).await?;

    print_json_pretty(PrintableQueryStoreResponse {
        key: key_hex,
        value: value.map(hex::encode),
        proof,
    })
}

fn print_json_pretty<T>(data: T) -> anyhow::Result<()>
where
    T: Serialize,
{
    let json = data.to_json_string_pretty()?;
    let colored = json.to_colored_json_auto()?;

    println!("{colored}");

    Ok(())
}
