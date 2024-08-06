use {
    crate::prompt::print_json_pretty,
    clap::{Parser, Subcommand},
    grug_client::Client,
    grug_jmt::Proof,
    grug_types::{Addr, Binary, Hash256, QueryRequest},
    serde::Serialize,
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
    Code { hash: Hash256 },
    /// Enumerate all Wasm byte codes
    Codes {
        /// Start after this hash
        start_after: Option<Hash256>,
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
}

impl QueryCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let client = Client::connect(&self.node)?;

        let req = match self.subcmd {
            SubCmd::Info => QueryRequest::Info {},
            SubCmd::Balance { address, denom } => QueryRequest::Balance { address, denom },
            SubCmd::Balances {
                address,
                start_after,
                limit,
            } => QueryRequest::Balances {
                address,
                start_after,
                limit,
            },
            SubCmd::Supply { denom } => QueryRequest::Supply { denom },
            SubCmd::Supplies { start_after, limit } => {
                QueryRequest::Supplies { start_after, limit }
            },
            SubCmd::Code { hash } => QueryRequest::Code { hash },
            SubCmd::Codes { start_after, limit } => QueryRequest::Codes { start_after, limit },
            SubCmd::Account { address } => QueryRequest::Account { address },
            SubCmd::Accounts { start_after, limit } => {
                QueryRequest::Accounts { start_after, limit }
            },
            SubCmd::WasmRaw { contract, key_hex } => {
                // We interpret the input raw key as Hex encoded
                let key = Binary::from(hex::decode(key_hex)?);
                QueryRequest::WasmRaw { contract, key }
            },
            SubCmd::WasmSmart { contract, msg } => {
                // The input should be a JSON string, e.g. `{"config":{}}`
                let msg = serde_json::from_str(&msg)?;
                QueryRequest::WasmSmart { contract, msg }
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
