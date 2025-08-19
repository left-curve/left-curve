use {
    dango_genesis::GenesisOption,
    dango_mock_httpd::{get_mock_socket_addr, wait_for_server_ready},
    dango_testing::{Preset, TestAccounts, TestOption},
    grug::BlockCreation,
    indexer_client::HttpClient,
};

pub async fn setup_client_test() -> anyhow::Result<(HttpClient, TestAccounts)> {
    let port = get_mock_socket_addr();

    let (sx, rx) = tokio::sync::oneshot::channel();

    // Run server in separate thread with its own runtime
    std::thread::spawn(move || {
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
                println!("Error running mock HTTP server: {error}");
            }
        });
    });

    let accounts = rx.await?;

    wait_for_server_ready(port).await?;

    Ok((
        HttpClient::new(format!("http://localhost:{port}"))?,
        accounts,
    ))
}
