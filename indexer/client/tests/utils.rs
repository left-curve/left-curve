use {
    dango_genesis::GenesisOption,
    dango_mock_httpd::{get_mock_socket_addr, wait_for_server_ready},
    dango_testing::{Preset, TestAccounts, TestOption},
    grug::{BlockCreation, setup_tracing_subscriber},
    indexer_client::HttpClient,
};

pub async fn setup_client_test() -> anyhow::Result<(HttpClient, TestAccounts)> {
    setup_tracing_subscriber(tracing::Level::INFO);

    let port = get_mock_socket_addr();

    let (sx, rx) = tokio::sync::oneshot::channel();

    // Spawn server in separate thread with its own runtime
    let _server_handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tracing::info!("Starting mock HTTP server on port {port}");

            if let Err(error) = dango_mock_httpd::run_with_callback(
                port,
                BlockCreation::OnBroadcast,
                None,
                TestOption::default(),
                GenesisOption::preset_test(),
                true,
                None,
                |accounts, _, _, _| {
                    sx.send(accounts).unwrap();
                },
            )
            .await
            {
                tracing::error!("Error running mock HTTP server: {error}");
            }
        });
    });

    let accounts = rx.await?;

    wait_for_server_ready(port).await?;

    Ok((
        HttpClient::new(&format!("http://localhost:{port}")),
        accounts,
    ))
}
