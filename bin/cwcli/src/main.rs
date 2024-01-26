mod format;
mod key;
mod query;
mod tx;

use {
    crate::{key::KeyCmd, query::QueryCmd, tx::TxCmd},
    clap::Parser,
    std::path::PathBuf,
};

#[derive(Parser)]
#[command(author, version, about, next_display_order = None)]
struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Tendermint RPC address
    #[arg(long, global = true, default_value = "http://127.0.0.1:26657")]
    pub node: String,

    /// Directory for storing keys
    #[arg(long, global = true, default_value = "~/.cwcli/keys")]
    pub key_dir: PathBuf,
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
    #[command(subcommand, next_display_order = None)]
    Key(KeyCmd),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Query(cmd) => cmd.run(&cli.node).await,
        Command::Tx(cmd) => cmd.run(),
        Command::Key(cmd) => cmd.run(),
    }
}
