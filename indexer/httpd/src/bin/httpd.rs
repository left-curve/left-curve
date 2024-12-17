use {
    clap::Parser,
    indexer_httpd::{error::Error, server::run_server},
    tracing_subscriber::EnvFilter,
};

#[derive(Parser)]
pub struct Cli {
    #[arg(long, default_value = "0.0.0.0")]
    ip: String,

    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// The database url
    #[arg(long, default_value = "postgres://localhost")]
    indexer_database_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        //.pretty()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    run_server(Some(&cli.ip), Some(cli.port)).await?;
    Ok(())
}
