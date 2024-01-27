use {
    clap::Parser,
    cw_keyring::print_json_pretty,
    tendermint::block::Height,
    tendermint_rpc::{Client, HttpClient},
};

#[derive(Parser)]
pub enum TendermintCmd {
    /// Get Tendermint status, including node info, pubkey, latest block hash,
    /// app hash, block height, and time
    Status,
    /// Find transaction by hash
    Tx {
        /// Transaction hash
        hash: String,
    },
    /// Get block at a given height
    Block {
        /// Block height [default: latest]
        height: Option<u64>,
    },
    /// Get block, including transaction execution results and events, at a given
    /// height
    BlockResults {
        /// Block height [default: latest]
        height: Option<u64>,
    },
}

impl TendermintCmd {
    pub async fn run(self, rpc_addr: &str) -> anyhow::Result<()> {
        let client = HttpClient::new(rpc_addr)?;
        match self {
            TendermintCmd::Status => query_status(client).await,
            TendermintCmd::Tx {
                hash,
            } => query_tx(client, hash).await,
            TendermintCmd::Block {
                height,
            } => query_block(client, height).await,
            TendermintCmd::BlockResults {
                height,
            } => query_block_results(client, height).await,
        }
    }
}

async fn query_status(client: impl Client + Sync) -> anyhow::Result<()> {
    let res = client.status().await?;
    print_json_pretty(res)
}

async fn query_tx(client: impl Client + Sync, hash_str: String) -> anyhow::Result<()> {
    let hash_bytes = hex::decode(&hash_str)?;
    let res = client.tx(hash_bytes.try_into()?, false).await?;
    print_json_pretty(res)
}

async fn query_block(client: impl Client + Sync, height: Option<u64>) -> anyhow::Result<()> {
    let res = match height {
        Some(height) => client.block(Height::try_from(height)?).await?,
        None => client.latest_block().await?,
    };
    print_json_pretty(res)
}

async fn query_block_results(
    client: impl Client + Sync,
    height: Option<u64>,
) -> anyhow::Result<()> {
    let res = match height {
        Some(height) => client.block_results(Height::try_from(height)?).await?,
        None => client.latest_block_results().await?,
    };
    print_json_pretty(res)
}
