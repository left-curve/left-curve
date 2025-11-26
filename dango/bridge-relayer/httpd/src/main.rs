use {
    dango_bridge_relayer_httpd::{error::Error, server},
    metrics_exporter_prometheus::PrometheusBuilder,
};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize metrics handler.
    // This should be done as soon as possible to capture all events.
    let metrics_handler = PrometheusBuilder::new().install_recorder()?;

    // Run the server
    let server = server::run_server("127.0.0.1", 8080, None);
    let metrics = server::run_metrics_server("127.0.0.1", 8081, metrics_handler);

    tokio::try_join!(server, metrics)?;

    Ok(())
}
