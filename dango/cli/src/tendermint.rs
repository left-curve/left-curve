use {
    crate::{config::Config, home_directory::HomeDirectory, prompt::print_json_pretty},
    clap::{Parser, Subcommand},
    config_parser::parse_config,
    grug_client::TendermintRpcClient,
    grug_types::{BlockClient, Hash, SearchTxClient},
    std::str::FromStr,
};

#[derive(Parser)]
pub struct TendermintCmd {
    #[command(subcommand)]
    subcmd: SubCmd,
}

#[derive(Subcommand)]
pub enum SubCmd {
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
}

impl TendermintCmd {
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        // Parse the config file.
        let cfg: Config = parse_config(app_dir.config_file())?;

        // Create Grug client via Tendermint RPC.
        let client = TendermintRpcClient::new(cfg.tendermint.rpc_addr.as_str())?;

        match self.subcmd {
            SubCmd::Status => {
                let res = client.status().await?;
                print_json_pretty(res)
            },
            SubCmd::Tx { hash } => {
                // Cast the hex string to uppercase, so that users can use either upper or
                // lowercase on the CLI.
                let hash = Hash::from_str(&hash.to_ascii_uppercase())?;
                let res = client.search_tx(hash).await?;
                print_json_pretty(res)
            },
            SubCmd::Block { height } => {
                let res = client.query_block_outcome(height).await?;
                print_json_pretty(res)
            },
        }
    }
}
