use {
    crate::prompt::print_json_pretty,
    anyhow::bail,
    clap::Parser,
    cw_std::{from_json, to_json, Addr, Hash, QueryRequest, QueryResponse},
    std::{fs::File, io::Write, path::Path},
    tendermint_rpc::{Client, HttpClient},
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
}

impl QueryCmd {
    pub async fn run(self, rpc_addr: &str) -> anyhow::Result<()> {
        let client = HttpClient::new(rpc_addr)?;
        match self {
            QueryCmd::Info => query_info(client).await,
            QueryCmd::Balance {
                address,
                denom,
            } => query_balance(client, address, denom).await,
            QueryCmd::Balances {
                address,
                start_after,
                limit,
            } => query_balances(client, address, start_after, limit).await,
            QueryCmd::Supply {
                denom,
            } => query_supply(client, denom).await,
            QueryCmd::Supplies {
                start_after,
                limit,
            } => query_supplies(client, start_after, limit).await,
            QueryCmd::Code {
                hash,
            } => query_code(client, hash).await,
            QueryCmd::Codes {
                start_after,
                limit,
            } => query_codes(client, start_after, limit).await,
            QueryCmd::Account {
                address,
            } => query_account(client, address).await,
            QueryCmd::Accounts {
                start_after,
                limit,
            } => query_accounts(client, start_after, limit).await,
            QueryCmd::WasmRaw {
                contract,
                key_hex,
            } => query_wasm_raw(client, contract, key_hex).await,
            QueryCmd::WasmSmart {
                contract,
                msg,
            } => query_wasm_smart(client, contract, msg).await,
        }
    }
}

async fn query_info(client: impl Client + Sync) -> anyhow::Result<()> {
    let res = do_abci_query(client, QueryRequest::Info {}).await?;
    print_json_pretty(res.as_info())
}

async fn query_balance(
    client:  impl Client + Sync,
    address: Addr,
    denom:   String,
) -> anyhow::Result<()> {
    let res = do_abci_query(client, QueryRequest::Balance { address, denom }).await?;
    print_json_pretty(res.as_balance())
}

async fn query_balances(
    client:      impl Client + Sync,
    address:     Addr,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> anyhow::Result<()> {
    let res = do_abci_query(client, QueryRequest::Balances { address, start_after, limit }).await?;
    print_json_pretty(res.as_balances())
}

async fn query_supply(client: impl Client + Sync, denom: String) -> anyhow::Result<()> {
    let res = do_abci_query(client, QueryRequest::Supply { denom }).await?;
    print_json_pretty(res.as_supply())
}

async fn query_supplies(
    client:      impl Client + Sync,
    start_after: Option<String>,
    limit:       Option<u32>,
) -> anyhow::Result<()> {
    let res = do_abci_query(client, QueryRequest::Supplies { start_after, limit }).await?;
    print_json_pretty(res.as_supplies())
}

async fn query_code(client: impl Client + Sync, hash: Hash) -> anyhow::Result<()> {
    // we will be writing the wasm byte code to $(pwd)/${hash}.wasm
    // first check if the file already exists. throw if it does
    let filename = format!("{hash}.wasm");
    if Path::new(&filename).exists() {
        bail!("the file {filename} already exists!");
    }

    // make the query
    let res = do_abci_query(client, QueryRequest::Code { hash }).await?;

    // write the bytes to the file
    // this creates a new file if not exists, an overwrite if exists
    let mut file = File::create(&filename)?;
    file.write_all(&res.as_code())?;

    println!("wasm byte code written to {filename}");

    Ok(())
}

async fn query_codes(
    client:      impl Client + Sync,
    start_after: Option<Hash>,
    limit:       Option<u32>,
) -> anyhow::Result<()> {
    let res = do_abci_query(client, QueryRequest::Codes { start_after, limit }).await?;
    print_json_pretty(res.as_codes())
}

async fn query_account(client: impl Client + Sync, address: Addr) -> anyhow::Result<()> {
    let res = do_abci_query(client, QueryRequest::Account { address }).await?;
    print_json_pretty(res.as_account())
}

async fn query_accounts(
    client:      impl Client + Sync,
    start_after: Option<Addr>,
    limit:       Option<u32>,
) -> anyhow::Result<()> {
    let res = do_abci_query(client, QueryRequest::Accounts { start_after, limit }).await?;
    print_json_pretty(res.as_accounts())
}

async fn query_wasm_raw(
    client:   impl Client + Sync,
    contract: Addr,
    key_hex:  String,
) -> anyhow::Result<()> {
    // we interpret the input raw key as Hex encoded
    let key = hex::decode(&key_hex)?.into();
    let res = do_abci_query(client, QueryRequest::WasmRaw { contract, key }).await?;
    print_json_pretty(res.as_wasm_raw())
}

async fn query_wasm_smart(
    client:   impl Client + Sync,
    contract: Addr,
    msg:      String,
) -> anyhow::Result<()> {
    // the input should be a JSON string, e.g. `{"config":{}}`
    let msg = msg.into_bytes().into();
    let res = do_abci_query(client, QueryRequest::WasmSmart { contract, msg }).await?;
    print_json_pretty(res.as_wasm_smart())
}

// NOTE: the App's current implementation of ABCI Query method ignores `path`,
// `height`, and `prove` fields, and interpret `data` and JSON-encoded QueryRequest.
pub async fn do_abci_query(
    client: impl Client + Sync,
    req:    QueryRequest,
) -> anyhow::Result<QueryResponse> {
    let res = client.abci_query(None, to_json(&req)?, None, false).await?;
    if res.code.is_err() {
        bail!(
            "query failed! codespace = {}, code = {}, log = {}",
            res.codespace,
            res.code.value(),
            res.log
        );
    }
    from_json(&res.value).map_err(Into::into)
}
