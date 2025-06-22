use {
    anyhow::Ok,
    assertor::*,
    dango_genesis::GenesisOption,
    dango_mock_httpd::{BlockCreation, TestOption, get_mock_socket_addr, wait_for_server_ready},
    dango_testing::Preset,
    dango_types::config::AppConfig,
    grug::{QueryClientExt, setup_tracing_subscriber},
    indexer_client::HttpClient,
    indexer_httpd::server::run_metrics_server,
    metrics_exporter_prometheus::PrometheusBuilder,
};

#[tokio::test]
async fn metrics_are_available() -> anyhow::Result<()> {
    setup_tracing_subscriber(tracing::Level::ERROR);

    let metrics_handler = PrometheusBuilder::new().install_recorder()?;

    let port = get_mock_socket_addr();
    let metrics_port = get_mock_socket_addr();

    // Spawn server in separate thread with its own runtime
    let _server_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tracing::info!("Starting mock HTTP server on port {port}");

            if let Err(error) = dango_mock_httpd::run(
                port,
                BlockCreation::OnBroadcast,
                None,
                TestOption::default(),
                GenesisOption::preset_test(),
                true,
                None,
            )
            .await
            {
                tracing::error!("Error running mock HTTP server: {error}");
            }
        });
    });

    wait_for_server_ready(port).await?;

    // Spawn server in separate thread with its own runtime
    let _server_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tracing::info!("Starting metrics HTTP server on port {metrics_port}");
            run_metrics_server("127.0.0.1", metrics_port, metrics_handler).await
        })
    });

    let client = HttpClient::new(&format!("http://localhost:{port}"));
    let res = client.query_app_config::<AppConfig>(None).await;

    assert_that!(res).is_ok();

    let metrics_client = reqwest::Client::new();
    let metrics_response = metrics_client
        .get(format!("http://localhost:{metrics_port}/metrics"))
        .send()
        .await?;

    let metrics_body = metrics_response.text().await?;

    // Uncomment the line below to print the metrics response for debugging
    // println!("Metrics response:\n{}", metrics_body);

    assert_that!(metrics_body).contains("graphql_requests_total");
    assert_that!(metrics_body).contains("http_requests_total");

    Ok(())
}
