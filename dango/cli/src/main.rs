mod config;
mod db;
mod home_directory;
mod init;
mod keys;
mod prompt;
mod query;
mod start;
mod tx;

use {
    crate::{
        db::DbCmd, home_directory::HomeDirectory, init::InitCmd, keys::KeysCmd, query::QueryCmd,
        start::StartCmd, tx::TxCmd,
    },
    clap::Parser,
    std::path::PathBuf,
    tracing::metadata::LevelFilter,
};

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    /// Directory for the physical database [default: ~/.dango]
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
    /// Manage the database
    #[command(subcommand, next_display_order = None)]
    Db(DbCmd),

    /// Manage keys
    #[command(subcommand, next_display_order = None)]
    Keys(KeysCmd),

    /// Initialize the home directory.
    Init(InitCmd),

    /// Make a query [alias: q]
    #[command(next_display_order = None, alias = "q")]
    Query(QueryCmd),

    /// Start the node
    Start(StartCmd),

    /// Send transactions
    #[command(next_display_order = None)]
    Tx(TxCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments.
    let cli = Cli::parse();

    // Find the home directory from the CLI `--home` flag.
    let app_dir = HomeDirectory::new_or_default(cli.home)?;

    // Set up tracing.
    tracing_subscriber::fmt()
        .with_max_level(cli.tracing_level)
        .init();

    match cli.command {
        Command::Db(cmd) => cmd.run(app_dir),
        Command::Keys(cmd) => cmd.run(app_dir.keys_dir()),
        Command::Init(cmd) => cmd.run(&app_dir),
        Command::Query(cmd) => cmd.run(app_dir).await,
        Command::Start(cmd) => cmd.run(app_dir).await,
        Command::Tx(cmd) => cmd.run(app_dir).await,
    }
}
