mod config;
mod home_directory;
mod keys;
mod prompt;
mod query;
mod tx;

use {
    crate::home_directory::HomeDirectory,
    clap::{Parser, Subcommand},
    std::path::PathBuf,
};

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    /// Directory for the Dango client's home [default: ~/.dango]
    #[arg(long, global = true)]
    home: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage keys
    #[command(subcommand, next_display_order = None)]
    Keys(keys::KeysCmd),

    /// Make a query [alias: q]
    #[command(next_display_order = None, alias = "q")]
    Query(query::QueryCmd),

    /// Send transactions
    #[command(next_display_order = None)]
    Tx(tx::TxCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let app_dir = HomeDirectory::new_or_default(cli.home)?;

    match cli.command {
        Command::Keys(cmd) => cmd.run(app_dir.keys_dir())?,
        Command::Query(cmd) => cmd.run(app_dir).await?,
        Command::Tx(cmd) => cmd.run(app_dir).await?,
    }

    Ok(())
}
