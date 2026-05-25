use {
    assertor::*,
    dango_testing::{mock_httpd_get_socket_addr, mock_httpd_wait_for_server_ready},
    indexer_metrics::run_metrics_server,
    metrics_exporter_prometheus::PrometheusBuilder,
    std::thread,
};

#[tokio::test]
async fn metrics_server_exposes_metrics() -> anyhow::Result<()> {
    let metrics_handler = PrometheusBuilder::new().install_recorder()?;
    let port = mock_httpd_get_socket_addr();

    // Start the metrics server in a separate thread
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            run_metrics_server("127.0.0.1", port, metrics_handler)
                .await
                .unwrap();
        });
    });

    mock_httpd_wait_for_server_ready(port).await?;

    let metrics_client = reqwest::Client::new();
    // This create a metric that we can use to test the metrics server
    metrics_client
        .get(format!("http://localhost:{port}/metrics"))
        .send()
        .await?;

    let metrics_response = metrics_client
        .get(format!("http://localhost:{port}/metrics"))
        .send()
        .await?;

    let metrics_body = metrics_response.text().await?;

    // Uncomment the line below to print the metrics response for debugging
    // println!("Metrics response:\n{}", metrics_body);

    // `http_requests_total` was removed in actix-web-metrics 0.4, which switched to
    // OpenTelemetry Semantic Conventions. The equivalent metric is now
    // `http.server.request.duration` (rendered as `http_server_request_duration` in
    // Prometheus format).
    // assert_that!(metrics_body).contains("http_requests_total");
    assert_that!(metrics_body).contains("http_server_request_duration");

    Ok(())
}
