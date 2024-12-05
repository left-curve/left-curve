mod db;
mod query;
mod start;

use {
    crate::{db::DbCmd, query::QueryCmd, start::StartCmd},
    anyhow::anyhow,
    clap::Parser,
    home::home_dir,
    std::path::PathBuf,
    tracing::metadata::LevelFilter,
};

// relative to user home directory (~)
const DEFAULT_APP_DIR: &str = ".dango";

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
    /// Manage the database
    #[command(subcommand, next_display_order = None)]
    Db(DbCmd),

    /// Make a query [alias: q]
    #[command(next_display_order = None, alias = "q")]
    Query(QueryCmd),

    /// Start the node
    Start(StartCmd),
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

    match cli.command {
        Command::Db(cmd) => cmd.run(data_dir),
        Command::Query(cmd) => cmd.run().await,
        Command::Start(cmd) => cmd.run(data_dir).await,
    }
}
