mod keys;
mod prompt;
mod query;
mod reset;
mod start;
mod tendermint;
mod tx;

use {
    crate::{
        keys::KeysCmd, query::QueryCmd, reset::ResetCmd, start::StartCmd,
        tendermint::TendermintCmd, tx::TxCmd,
    },
    anyhow::anyhow,
    clap::Parser,
    home::home_dir,
    std::path::PathBuf,
    tracing::metadata::LevelFilter,
};

// relative to user home directory (~)
const DEFAULT_APP_DIR: &str = ".grug";

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    /// Directory for the physical database
    #[arg(long, global = true)]
    home: Option<PathBuf>,

    /// Logging verbosity: error|warn|info|debug|trace
    #[arg(long, global = true, default_value = "info")]
    tracing_level: LevelFilter,

    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Manage keys [alias: k]
    #[command(subcommand, next_display_order = None, alias = "k")]
    Keys(KeysCmd),

    /// Make a query [alias: q]
    #[command(next_display_order = None, alias = "q")]
    Query(QueryCmd),

    /// Delete node data
    Reset(ResetCmd),

    /// Start the node
    Start(StartCmd),

    /// Tendermint status and queries [alias: tm]
    #[command(next_display_order = None, alias = "tm")]
    Tendermint(TendermintCmd),

    /// Send a transaction
    #[command(next_display_order = None)]
    Tx(TxCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(cli.tracing_level)
        .init();

    let app_dir = if let Some(dir) = cli.home {
        dir
    } else {
        home_dir()
            .ok_or(anyhow!("Failed to find home directory"))?
            .join(DEFAULT_APP_DIR)
    };
    let data_dir = app_dir.join("data");
    let keys_dir = app_dir.join("keys");

    match cli.command {
        Command::Keys(cmd) => cmd.run(keys_dir),
        Command::Query(cmd) => cmd.run().await,
        Command::Reset(cmd) => cmd.run(data_dir),
        Command::Start(cmd) => cmd.run(data_dir).await,
        Command::Tendermint(cmd) => cmd.run().await,
        Command::Tx(cmd) => cmd.run(keys_dir).await,
    }
}
