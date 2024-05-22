use {
    crate::prompt::print_json_pretty,
    clap::Parser,
    grug_sdk::Client,
};

#[derive(Parser)]
pub enum TendermintCmd {
    /// Get Tendermint status, including node info, public key, latest block hash,
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
        let client = Client::connect(rpc_addr)?;
        match self {
            TendermintCmd::Status => print_json_pretty(client.status().await?),
            TendermintCmd::Tx {
                hash,
            } => print_json_pretty(client.tx(&hash).await?),
            TendermintCmd::Block {
                height,
            } => print_json_pretty(client.block(height).await?),
            TendermintCmd::BlockResults {
                height,
            } => print_json_pretty(client.block_result(height).await?),
        }
    }
}
