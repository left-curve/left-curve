use {
    assertor::*,
    dango_genesis::GenesisOption,
    dango_primitives::QueryClientExt,
    dango_sdk::HttpClient,
    dango_testing::{
        BlockCreation, Preset, TestOption, mock_httpd_get_socket_addr,
        mock_httpd_wait_for_server_ready,
    },
    dango_types::config::AppConfig,
};

#[tokio::test]
async fn graphql_returns_config() -> anyhow::Result<()> {
    let port = mock_httpd_get_socket_addr();

    // Spawn server in separate thread with its own runtime
    let _server_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tracing::info!("Starting mock HTTP server on port {port}");

            if let Err(error) = dango_testing::mock_httpd_run(
                port,
                BlockCreation::OnBroadcast,
                None,
                TestOption::default(),
                GenesisOption::preset_test(),
                None,
            )
            .await
            {
                // Using println! so even without `setup_tracing_subscriber` we can see the error
                println!("Error running mock HTTP server: {error}");
            }
        });
    });

    mock_httpd_wait_for_server_ready(port).await?;

    let client = HttpClient::new(format!("http://localhost:{port}"))?;
    let res = client.query_app_config::<AppConfig>(None).await;

    assert_that!(res).is_ok();

    Ok(())
}
