use {
    crate::prompt::print_json_pretty,
    clap::{Parser, Subcommand},
    grug_sdk::Client,
    grug_types::Hash,
    std::str::FromStr,
};

#[derive(Parser)]
pub struct TendermintCmd {
    /// Tendermint RPC address
    #[arg(long, global = true, default_value = "http://127.0.0.1:26657")]
    node: String,

    #[command(subcommand)]
    subcmd: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
    /// Node status
    Status {},
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
}

impl TendermintCmd {
    pub async fn run(self) -> anyhow::Result<()> {
        let client = Client::connect(&self.node)?;
        match self.subcmd {
            SubCmd::Status {} => {
                let res = client.query_status().await?;
                print_json_pretty(res)
            },
            SubCmd::Tx { hash } => {
                // Cast the hex string to uppercase, so that users can use
                // either upper or lowercase on the CLI.
                let hash = Hash::from_str(&hash.to_ascii_uppercase())?;
                let res = client.query_tx(hash).await?;
                print_json_pretty(res)
            },
            SubCmd::Block { height } => {
                let res = client.query_block_result(height).await?;
                print_json_pretty(res)
            },
        }
    }
}
