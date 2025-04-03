use {
    crate::{config::Config, home_directory::HomeDirectory},
    clap::{Parser, Subcommand},
    colored_json::ToColoredJson,
    config_parser::parse_config,
    grug_client::RpcClient,
    grug_types::{
        Addr, Binary, BlockClient, Bound, Denom, Hash, Hash256, JsonDeExt, JsonSerExt, Proof,
        Query, QueryAppClient, QueryWasmSmartRequest, SearchTxClient,
    },
    serde::Serialize,
    std::str::FromStr,
};

#[derive(Parser)]
pub struct QueryCmd {
    /// The block height at which to perform queries [default: last finalized height]
    #[arg(long)]
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
    /// Query the application-specific configuration
    AppConfig,
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
        key: String,
    },
    /// Enumerate key-value pairs in a contract's internal state.
    WasmScan {
        /// Contract address
        contract: Addr,
        /// Mimimum bound in hex encoding
        min: Option<String>,
        /// Maximum bound in hex encoding
        max: Option<String>,
        /// Maximum number of records to collect
        limit: Option<u32>,
        /// Use exclusive minimum bound
        #[arg(long, default_value_t = false)]
        min_exclusive: bool,
        /// Use inclusive maximum bound
        #[arg(long, default_value_t = false)]
        max_inclusive: bool,
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
        /// Key in b64 encoding
        key: String,
        /// Whether to request Merkle proof for raw store queries [default: false]
        #[arg(long, default_value_t = false)]
        prove: bool,
    },
}

impl QueryCmd {
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        // Parse the config file.
        let cfg: Config = parse_config(app_dir.config_file())?;

        // Create Grug client via Tendermint RPC.
        let client = RpcClient::new(cfg.tendermint.rpc_addr.as_str())?;

        let req = match self.subcmd {
            SubCmd::Status => {
                let res = client.status().await?;
                return print_json_pretty(res);
            },
            SubCmd::Tx { hash } => {
                return query_tx(&client, &hash).await;
            },
            SubCmd::Block { height } => {
                return query_block(&client, height).await;
            },
            SubCmd::Config => Query::config(),
            SubCmd::AppConfig => Query::app_config(),
            SubCmd::Balance { address, denom } => {
                let denom = Denom::try_from(denom)?;
                Query::balance(address, denom)
            },
            SubCmd::Balances {
                address,
                start_after,
                limit,
            } => {
                let start_after = start_after.map(Denom::try_from).transpose()?;
                Query::balances(address, start_after, limit)
            },
            SubCmd::Supply { denom } => {
                let denom = Denom::try_from(denom)?;
                Query::supply(denom)
            },
            SubCmd::Supplies { start_after, limit } => {
                let start_after = start_after.map(Denom::try_from).transpose()?;
                Query::supplies(start_after, limit)
            },
            SubCmd::Code { hash } => Query::code(hash),
            SubCmd::Codes { start_after, limit } => Query::codes(start_after, limit),
            SubCmd::Contract { address } => Query::contract(address),
            SubCmd::Contracts { start_after, limit } => Query::contracts(start_after, limit),
            SubCmd::WasmRaw { contract, key } => {
                // We interpret the input raw key as Hex encoded
                let key = Binary::from(hex::decode(key)?);
                Query::wasm_raw(contract, key)
            },
            SubCmd::WasmScan {
                contract,
                min,
                max,
                limit,
                min_exclusive,
                max_inclusive,
            } => {
                let min = min
                    .map(|min| -> anyhow::Result<_> {
                        let min = Binary::from_inner(hex::decode(min)?);
                        if min_exclusive {
                            Ok(Bound::Exclusive(min))
                        } else {
                            Ok(Bound::Inclusive(min))
                        }
                    })
                    .transpose()?;
                let max = max
                    .map(|max| -> anyhow::Result<_> {
                        let max = Binary::from_inner(hex::decode(max)?);
                        if max_inclusive {
                            Ok(Bound::Inclusive(max))
                        } else {
                            Ok(Bound::Exclusive(max))
                        }
                    })
                    .transpose()?;
                Query::wasm_scan(contract, min, max, limit)
            },
            SubCmd::WasmSmart { contract, msg } => {
                // The input should be a JSON string, e.g. `{"config":{}}`
                let msg = msg.deserialize_json()?;
                Query::WasmSmart(QueryWasmSmartRequest { contract, msg })
            },
            SubCmd::Store { key, prove } => {
                return query_store(&client, key, self.height, prove).await;
            },
        };

        client
            .query_chain(req, self.height)
            .await
            .and_then(print_json_pretty)
    }
}

// ------------------------------------ tx -------------------------------------

async fn query_tx(client: &RpcClient, hash: &str) -> anyhow::Result<()> {
    // Cast the hex string to uppercase, so that users can use either upper or
    // lowercase on the CLI.
    let hash = Hash::from_str(&hash.to_ascii_uppercase())?;
    let res = client.search_tx(hash).await?;

    print_json_pretty(res)
}

// ----------------------------------- block -----------------------------------

async fn query_block(client: &RpcClient, height: Option<u64>) -> anyhow::Result<()> {
    let res = client.query_block_result(height).await?;

    print_json_pretty(res)
}

// ----------------------------------- store -----------------------------------

#[derive(Serialize)]
struct PrintableQueryStoreResponse {
    key: String,
    value: Option<String>,
    proof: Option<Proof>,
}

async fn query_store(
    client: &RpcClient,
    key_b64: String,
    height: Option<u64>,
    prove: bool,
) -> anyhow::Result<()> {
    let key = Binary::from_str(&key_b64)?;
    let (value, proof) = client.query_store(key, height, prove).await?;

    print_json_pretty(PrintableQueryStoreResponse {
        key: key_b64,
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
