use {
    clap::Parser,
    grug_httpd::{context::Context, graphql, server},
    std::sync::Arc,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    ip: String,

    /// Port to bind to
    #[arg(long, default_value = "8080")]
    port: u16,

    /// CORS allowed origin
    #[arg(long)]
    cors_origin: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a mock grug app for demonstration
    // In a real application, you would create an actual grug app instance
    let grug_app = Arc::new(MockGrugApp);
    let context = Context::new(grug_app);

    println!("Starting HTTP server on {}:{}", args.ip, args.port);

    server::run_server(
        args.ip,
        args.port,
        args.cors_origin,
        context,
        server::config_app,
        graphql::build_schema,
    )
    .await?;

    Ok(())
}

// Mock implementation for demonstration
struct MockGrugApp;

#[async_trait::async_trait]
impl grug_httpd::traits::QueryApp for MockGrugApp {
    async fn query_app(
        &self,
        _raw_req: grug_types::Query,
        _height: Option<u64>,
    ) -> grug_app::AppResult<grug_types::QueryResponse> {
        Ok(grug_types::QueryResponse::AppConfig(
            grug_types::Json::null(),
        ))
    }

    async fn query_store(
        &self,
        _key: &[u8],
        _height: Option<u64>,
        _prove: bool,
    ) -> grug_app::AppResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        Ok((Some(b"mock_value".to_vec()), None))
    }

    async fn simulate(
        &self,
        _unsigned_tx: grug_types::UnsignedTx,
    ) -> grug_app::AppResult<grug_types::TxOutcome> {
        Ok(grug_types::TxOutcome {
            gas_limit: 0,
            gas_used: 0,
            result: Ok(()),
            events: grug_types::TxEvents::default(),
        })
    }

    async fn chain_id(&self) -> grug_app::AppResult<String> {
        Ok("test-chain".to_string())
    }

    async fn last_finalized_block(&self) -> grug_app::AppResult<grug_types::BlockInfo> {
        Ok(grug_types::BlockInfo {
            height: 1,
            timestamp: grug_types::Timestamp::from_seconds(0),
            hash: grug_types::Hash256::ZERO,
        })
    }
}
