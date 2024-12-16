use {
    clap::Parser,
    indexer_httpd::{error::Error, server::run_server},
    tracing::metadata::LevelFilter,
};

#[derive(Parser)]
pub struct Cli {
    #[arg(long, default_value = "0.0.0.0")]
    ip: String,

    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Logging verbosity: error|warn|info|debug|trace
    #[arg(long, global = true, default_value = "info")]
    tracing_level: LevelFilter,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_max_level(cli.tracing_level)
        .init();
    run_server(Some(&cli.ip), Some(cli.port)).await?;
    Ok(())
}
