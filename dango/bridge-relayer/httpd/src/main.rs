use {
    dango_bridge_relayer_httpd::{context::Context, error::Error, server},
    dango_types::{bitcoin::QueryConfigRequest, config::AppConfig},
    grug::{ClientWrapper, QueryClientExt},
    indexer_client::HttpClient,
    metrics_exporter_prometheus::PrometheusBuilder,
    std::sync::Arc,
};

const DANGO_URL: &str = "https://api-testnet.dango.zone/";

#[actix_web::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize metrics handler.
    // This should be done as soon as possible to capture all events.
    let metrics_handler = PrometheusBuilder::new().install_recorder()?;

    // Initialize Dango client.
    let dango_client = ClientWrapper::new(Arc::new(HttpClient::new(DANGO_URL)?));

    // Query the bitcoin bridge contract address.
    let bitcoin_bridge = dango_client
        .query_app_config::<AppConfig>(None)
        .await?
        .addresses
        .bitcoin;

    // Load bitcoin bridge config from contract.
    let bridge_config = dango_client
        .query_wasm_smart(bitcoin_bridge, QueryConfigRequest {}, None)
        .await?;

    // Run the server
    let server = server::run_server("127.0.0.1", 8080, None, Context::new(bridge_config));
    let metrics = server::run_metrics_server("127.0.0.1", 8081, metrics_handler);

    tokio::try_join!(server, metrics)?;

    Ok(())
}
