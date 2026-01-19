use {
    anyhow::bail,
    dango_genesis::{Codes, Contracts, GenesisCodes},
    dango_proposal_preparer::ProposalPreparer,
    dango_testing::{TestAccounts, setup_suite_with_db_and_vm},
    grug_app::SimpleCommitment,
    grug_db_memory::MemDb,
    grug_testing::MockClient,
    grug_vm_rust::{ContractWrapper, RustVm},
    hyperlane_testing::MockValidatorSets,
    indexer_hooked::HookedIndexer,
    rand::Rng,
    std::{
        collections::HashSet,
        sync::{Arc, LazyLock, Mutex as StdMutex, mpsc},
        time::Duration,
    },
    tokio::{net::TcpStream, sync::Mutex},
};
pub use {
    dango_genesis::GenesisOption,
    dango_testing::{BridgeOp, Preset, TestOption},
    grug_testing::BlockCreation,
    indexer_httpd::error::Error,
};

pub async fn run(
    port: u16,
    block_creation: BlockCreation,
    cors_allowed_origin: Option<String>,
    test_opt: TestOption,
    genesis_opt: GenesisOption,
    database_url: Option<String>,
) -> Result<(), Error> {
    run_with_callback(
        port,
        block_creation,
        cors_allowed_origin,
        test_opt,
        genesis_opt,
        database_url,
        |_, _, _, _, _| {},
    )
    .await
}

pub async fn run_with_callback<C>(
    port: u16,
    block_creation: BlockCreation,
    cors_allowed_origin: Option<String>,
    test_opt: TestOption,
    genesis_opt: GenesisOption,
    database_url: Option<String>,
    callback: C,
) -> Result<(), Error>
where
    C: FnOnce(
            TestAccounts,
            Codes<ContractWrapper>,
            Contracts,
            MockValidatorSets,
            indexer_sql::context::Context,
        ) + Send
        + Sync,
{
    let indexer = indexer_sql::IndexerBuilder::default();

    let indexer = if let Some(url) = database_url {
        indexer.with_database_url(url)
    } else {
        indexer
            .with_memory_database()
            .with_database_max_connections(1)
    };

    let indexer = indexer.with_sqlx_pubsub().build().await?;

    let indexer_context = indexer.context.clone();

    let indexer_cache = indexer_cache::Cache::new_with_tempdir();
    let indexer_cache_context = indexer_cache.context.clone();

    let mut hooked_indexer = HookedIndexer::new();

    // Create a separate context for dango indexer (shares DB but has independent pubsub)
    let dango_context: dango_indexer_sql::context::Context = indexer
        .context
        .with_separate_pubsub()
        .await
        .map_err(|e| {
            indexer_sql::error::IndexerError::from(anyhow::anyhow!(
                "Failed to create separate context for dango indexer: {e}",
            ))
        })?
        .into();

    let dango_indexer = dango_indexer_sql::indexer::Indexer::new(dango_context.clone());

    let indexer_context_callback = indexer.context.clone();

    hooked_indexer.add_indexer(indexer_cache).await.unwrap();
    hooked_indexer.add_indexer(indexer).await.unwrap();
    hooked_indexer.add_indexer(dango_indexer).await.unwrap();

    let (suite, test, codes, contracts, mock_validator_sets) = setup_suite_with_db_and_vm(
        MemDb::<SimpleCommitment>::new(),
        RustVm::new(),
        ProposalPreparer::new([""], ""), // FIXME: endpoints and access token
        hooked_indexer,
        RustVm::genesis_codes(),
        test_opt,
        genesis_opt,
    );

    callback(
        test,
        codes,
        contracts,
        mock_validator_sets,
        indexer_context_callback,
    );

    let suite = Arc::new(Mutex::new(suite));

    let mock_client = MockClient::new_shared(suite.clone(), block_creation);

    let app = suite.lock().await.app.clone_without_indexer();

    let indexer_httpd_context = indexer_httpd::context::Context::new(
        indexer_cache_context,
        indexer_context.clone(),
        Arc::new(app),
        Arc::new(mock_client),
    );

    let indexer_clickhouse_context = dango_indexer_clickhouse::context::Context::new(
        "http://localhost:8123".to_string(),
        "default".to_string(),
        "default".to_string(),
        "default".to_string(),
    );

    let dango_httpd_context = dango_httpd::context::Context::new(
        indexer_httpd_context.clone(),
        indexer_clickhouse_context.clone(),
        dango_context,
        None,
    );

    let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    dango_httpd::server::run_server(
        "127.0.0.1",
        port,
        cors_allowed_origin,
        dango_httpd_context,
        shutdown_flag,
        None,
    )
    .await
}

/// Run the mock server with port 0 and send the actual bound port via a channel.
/// This is useful for tests that need to run in parallel without port conflicts.
pub async fn run_with_port_sender(
    block_creation: BlockCreation,
    cors_allowed_origin: Option<String>,
    test_opt: TestOption,
    genesis_opt: GenesisOption,
    database_url: Option<String>,
    port_sender: mpsc::Sender<u16>,
) -> Result<(), Error> {
    let indexer = indexer_sql::IndexerBuilder::default();

    let indexer = if let Some(url) = database_url {
        indexer.with_database_url(url)
    } else {
        indexer
            .with_memory_database()
            .with_database_max_connections(1)
    };

    let indexer = indexer.with_sqlx_pubsub().build().await?;

    let indexer_context = indexer.context.clone();

    let indexer_cache = indexer_cache::Cache::new_with_tempdir();
    let indexer_cache_context = indexer_cache.context.clone();

    let mut hooked_indexer = HookedIndexer::new();

    // Create a separate context for dango indexer (shares DB but has independent pubsub)
    let dango_context: dango_indexer_sql::context::Context = indexer
        .context
        .with_separate_pubsub()
        .await
        .map_err(|e| {
            indexer_sql::error::IndexerError::from(anyhow::anyhow!(
                "Failed to create separate context for dango indexer: {e}",
            ))
        })?
        .into();

    let dango_indexer = dango_indexer_sql::indexer::Indexer::new(dango_context.clone());

    hooked_indexer.add_indexer(indexer_cache).await.unwrap();
    hooked_indexer.add_indexer(indexer).await.unwrap();
    hooked_indexer.add_indexer(dango_indexer).await.unwrap();

    let (suite, _test, _codes, _contracts, _mock_validator_sets) = setup_suite_with_db_and_vm(
        MemDb::<SimpleCommitment>::new(),
        RustVm::new(),
        ProposalPreparer::new([""], ""), // FIXME: endpoints and access token
        hooked_indexer,
        RustVm::genesis_codes(),
        test_opt,
        genesis_opt,
    );

    let suite = Arc::new(Mutex::new(suite));

    let mock_client = MockClient::new_shared(suite.clone(), block_creation);

    let app = suite.lock().await.app.clone_without_indexer();

    let indexer_httpd_context = indexer_httpd::context::Context::new(
        indexer_cache_context,
        indexer_context.clone(),
        Arc::new(app),
        Arc::new(mock_client),
    );

    let indexer_clickhouse_context = dango_indexer_clickhouse::context::Context::new(
        "http://localhost:8123".to_string(),
        "default".to_string(),
        "default".to_string(),
        "default".to_string(),
    );

    let dango_httpd_context = dango_httpd::context::Context::new(
        indexer_httpd_context.clone(),
        indexer_clickhouse_context.clone(),
        dango_context,
        None,
    );

    let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    dango_httpd::server::run_server(
        "127.0.0.1",
        0, // Let the OS allocate an available port
        cors_allowed_origin,
        dango_httpd_context,
        shutdown_flag,
        Some(port_sender),
    )
    .await
}

/// Tracks ports already allocated to avoid collisions.
static USED_PORTS: LazyLock<StdMutex<HashSet<u16>>> =
    LazyLock::new(|| StdMutex::new(HashSet::new()));

/// Get a random port for the mock server.
///
/// Generates a random port in the range 20000-60000, ensuring it hasn't
/// been used by another test in this process.
pub fn get_mock_socket_addr() -> u16 {
    let mut rng = rand::thread_rng();
    let mut used = USED_PORTS.lock().unwrap();

    loop {
        let port = rng.gen_range(20000..60000);
        if used.insert(port) {
            return port;
        }
    }
}

/// Wait for the server to be ready to accept connections.
/// CI measurements showed server starts in ~50ms (2 attempts).
/// Using 30 attempts * 100ms = 3s max timeout for safety margin.
pub async fn wait_for_server_ready(port: u16) -> anyhow::Result<()> {
    for attempt in 1..=30 {
        match TcpStream::connect(format!("127.0.0.1:{port}")).await {
            Ok(_) => {
                tracing::info!("Server ready on port {port} after {attempt} attempts");
                return Ok(());
            },
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            },
        }
    }

    bail!("server failed to start on port {port} after 30 attempts (3s)")
}
