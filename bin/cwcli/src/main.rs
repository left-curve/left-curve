mod format;
mod key;
mod query;
mod tx;

use {
    crate::{key::KeyCmd, query::QueryCmd, tx::TxCmd},
    anyhow::anyhow,
    clap::Parser,
    cw_std::Addr,
    home::home_dir,
    std::path::PathBuf,
};

// relative to user home directory (~)
const DEFAULT_KEY_DIR: &str = ".cwcli/keys";

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Tendermint RPC address
    #[arg(long, global = true, default_value = "http://127.0.0.1:26657")]
    pub node: String,

    /// Directory for storing keys [default: ~/.cwcli/keys]
    #[arg(long, global = true)]
    pub key_dir: Option<PathBuf>,

    /// Name of the key to sign transactions
    #[arg(long, global = true)]
    pub key_name: Option<String>,

    /// Transaction sender address
    #[arg(long, global = true)]
    pub sender: Option<Addr>,

    /// Chain identifier [default: query from chain]
    #[arg(long, global = true)]
    pub chain_id: Option<String>,

    /// Account sequence number [default: query from chain]
    #[arg(long, global = true)]
    pub sequence: Option<u32>,
}

#[derive(Parser)]
enum Command {
    /// Make a query
    #[command(subcommand, next_display_order = None, alias = "q")]
    Query(QueryCmd),

    /// Send a transaction
    #[command(subcommand, next_display_order = None)]
    Tx(TxCmd),

    /// Manage keys
    #[command(subcommand, next_display_order = None, alias = "k")]
    Key(KeyCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let key_dir = if let Some(dir) = cli.key_dir {
        dir
    } else {
        let home_dir = home_dir().ok_or(anyhow!("Failed to find home directory"))?;
        home_dir.join(DEFAULT_KEY_DIR)
    };

    match cli.command {
        Command::Query(cmd) => cmd.run(&cli.node).await,
        Command::Tx(cmd) => {
            cmd.run(&cli.node, key_dir, cli.key_name, cli.sender, cli.chain_id, cli.sequence).await
        },
        Command::Key(cmd) => cmd.run(key_dir),
    }
}
