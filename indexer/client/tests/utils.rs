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
                None,
                |accounts, _, _, _, _| {
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

// Channel-based approach (commented out for comparison)
// This approach guarantees we get the port AFTER binding
// pub async fn setup_client_test_with_channel() -> anyhow::Result<(HttpClient, TestAccounts)> {
//     let (port_tx, port_rx) = mpsc::channel();
//     let (accounts_tx, accounts_rx) = tokio::sync::oneshot::channel();
//
//     std::thread::spawn(move || {
//         let rt = tokio::runtime::Runtime::new().unwrap();
//         rt.block_on(async {
//             if let Err(error) = dango_mock_httpd::run_with_port_sender(
//                 BlockCreation::OnBroadcast,
//                 None,
//                 TestOption::default(),
//                 GenesisOption::preset_test(),
//                 None,
//                 port_tx,
//             )
//             .await
//             {
//                 println!("Error running mock HTTP server: {error}");
//             }
//         });
//     });
//
//     // Wait for the actual bound port - this is sent AFTER server.bind()
//     let port = port_rx.recv()?;
//     tracing::info!("Server bound to port {port}");
//
//     // Server is already bound at this point, no need to wait
//     Ok((
//         HttpClient::new(format!("http://localhost:{port}"))?,
//         accounts_rx.await?,
//     ))
// }
