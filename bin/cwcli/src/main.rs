mod key;
mod prompt;
mod query;
mod tendermint;
mod tx;

use {
    crate::{key::KeyCmd, query::QueryCmd, tendermint::TendermintCmd, tx::TxCmd},
    anyhow::anyhow,
    clap::Parser,
    grug::Addr,
    home::home_dir,
    std::path::PathBuf,
};

// relative to user home directory (~)
const DEFAULT_KEY_DIR: &str = ".cwcli/keys";

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Tendermint RPC address
    #[arg(long, global = true, default_value = "http://127.0.0.1:26657")]
    node: String,

    /// Directory for storing keys [default: ~/.cwcli/keys]
    #[arg(long, global = true)]
    key_dir: Option<PathBuf>,

    /// Name of the key to sign transactions
    #[arg(long, global = true)]
    key_name: Option<String>,

    /// Transaction sender address
    #[arg(long, global = true)]
    sender: Option<Addr>,

    /// Chain identifier [default: query from chain]
    #[arg(long, global = true)]
    chain_id: Option<String>,

    /// Account sequence number [default: query from chain]
    #[arg(long, global = true)]
    sequence: Option<u32>,

    /// The block height at which to perform queries [default: last finalized height]
    #[arg(long, global = true)]
    height: Option<u64>,

    /// Whether to request Merkle proof for raw store queries [default: false]
    #[arg(long, global = true, default_value_t = false)]
    prove: bool,
}

#[derive(Parser)]
enum Command {
    /// Manage keys [alias: k]
    #[command(subcommand, next_display_order = None, alias = "k")]
    Key(KeyCmd),

    /// Make a query [alias: q]
    #[command(subcommand, next_display_order = None, alias = "q")]
    Query(QueryCmd),

    /// Interact with Tendermint consensus engine [alias: tm]
    #[command(subcommand, next_display_order = None, alias = "tm")]
    Tendermint(TendermintCmd),

    /// Send a transaction
    #[command(subcommand, next_display_order = None)]
    Tx(TxCmd),
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
        Command::Key(cmd) => cmd.run(key_dir),
        Command::Query(cmd) => cmd.run(&cli.node, cli.height, cli.prove).await,
        Command::Tendermint(cmd) => cmd.run(&cli.node).await,
        Command::Tx(cmd) => {
            cmd.run(&cli.node, key_dir, cli.key_name, cli.sender, cli.chain_id, cli.sequence).await
        },
    }
}
