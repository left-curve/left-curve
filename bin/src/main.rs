mod key;
mod prompt;
mod query;
mod start;
mod tendermint;
mod tx;

use {
    crate::{key::KeyCmd, query::QueryCmd, start::StartCmd, tendermint::TendermintCmd, tx::TxCmd},
    anyhow::anyhow,
    clap::Parser,
    grug::Addr,
    home::home_dir,
    std::path::PathBuf,
    tracing::metadata::LevelFilter,
};

// relative to user home directory (~)
const DEFAULT_APP_DIR: &str = ".grug";

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Tendermint RPC address
    #[arg(long, global = true, default_value = "http://127.0.0.1:26657")]
    node: String,

    /// Directory for the physical database
    #[arg(long, global = true)]
    app_dir: Option<PathBuf>,

    /// Name of the key to sign transactions
    #[arg(long, global = true)]
    key: Option<String>,

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

    /// Logging verbosity: error|warn|info|debug|trace
    #[arg(long, global = true, default_value = "info")]
    tracing_level: LevelFilter,
}

#[derive(Parser)]
enum Command {
    /// Manage keys [alias: k]
    #[command(subcommand, next_display_order = None, alias = "k")]
    Key(KeyCmd),

    /// Make a query [alias: q]
    #[command(subcommand, next_display_order = None, alias = "q")]
    Query(QueryCmd),

    /// Start the node
    #[command(subcommand, next_display_order = None)]
    Start(StartCmd),

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

    tracing_subscriber::fmt().with_max_level(cli.tracing_level).init();

    let app_dir = if let Some(dir) = cli.app_dir {
        dir
    } else {
        let home_dir = home_dir().ok_or(anyhow!("Failed to find home directory"))?;
        home_dir.join(DEFAULT_APP_DIR)
    };
    let data_dir = app_dir.join("data");
    let keys_dir = app_dir.join("keys");

    match cli.command {
        Command::Key(cmd) => cmd.run(keys_dir),
        Command::Query(cmd) => cmd.run(&cli.node, cli.height, cli.prove).await,
        Command::Start(cmd) => cmd.run(&cli.node, data_dir).await,
        Command::Tendermint(cmd) => cmd.run(&cli.node).await,
        Command::Tx(cmd) => {
            cmd.run(&cli.node, keys_dir, cli.key_name, cli.sender, cli.chain_id, cli.sequence).await
        },
    }
}
