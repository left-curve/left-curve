use {
    anyhow::bail,
    dango_genesis::{Codes, Contracts, GenesisCodes},
    dango_proposal_preparer::ProposalPreparer,
    dango_testing::{TestAccounts, setup_suite_with_db_and_vm},
    grug_db_memory::MemDb,
    grug_testing::MockClient,
    grug_vm_rust::{ContractWrapper, RustVm},
    hyperlane_testing::MockValidatorSets,
    indexer_hooked::HookedIndexer,
    std::{net::TcpListener, sync::Arc, time::Duration},
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
    keep_blocks: bool,
    database_url: Option<String>,
) -> Result<(), Error> {
    run_with_callback(
        port,
        block_creation,
        cors_allowed_origin,
        test_opt,
        genesis_opt,
        keep_blocks,
        database_url,
        |_, _, _, _| {},
    )
    .await
}

pub async fn run_with_callback<C>(
    port: u16,
    block_creation: BlockCreation,
    cors_allowed_origin: Option<String>,
    test_opt: TestOption,
    genesis_opt: GenesisOption,
    keep_blocks: bool,
    database_url: Option<String>,
    callback: C,
) -> Result<(), Error>
where
    C: FnOnce(TestAccounts, Codes<ContractWrapper>, Contracts, MockValidatorSets) + Send + Sync,
{
    let indexer = indexer_sql::IndexerBuilder::default();

    let indexer = if let Some(url) = database_url {
        indexer.with_database_url(url)
    } else {
        indexer
            .with_memory_database()
            .with_database_max_connections(1)
    };

    let indexer = indexer
        .with_keep_blocks(keep_blocks)
        .with_sqlx_pubsub()
        .with_tmpdir()
        .build()?;

    let indexer_context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let mut hooked_indexer = HookedIndexer::new();

    // Create a separate context for dango indexer (shares DB but has independent pubsub)
    let dango_context: dango_indexer_sql::context::Context = indexer
        .context
        .with_separate_pubsub()
        .await
        .map_err(|e| {
            Error::Indexer(indexer_sql::error::IndexerError::from(anyhow::anyhow!(
                "Failed to create separate context for dango indexer: {}",
                e
            )))
        })?
        .into();

    let dango_indexer = dango_indexer_sql::indexer::Indexer {
        runtime_handle: indexer_sql::indexer::RuntimeHandler::from_handle(
            indexer.handle.handle().clone(),
        ),
        context: dango_context.clone(),
    };
    hooked_indexer.add_indexer(indexer);
    hooked_indexer.add_indexer(dango_indexer);

    let (suite, test, codes, contracts, mock_validator_sets) = setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        ProposalPreparer::new(),
        hooked_indexer,
        RustVm::genesis_codes(),
        test_opt,
        genesis_opt,
    );

    callback(test, codes, contracts, mock_validator_sets);

    let suite = Arc::new(Mutex::new(suite));

    let mock_client = MockClient::new_shared(suite.clone(), block_creation);

    let app = suite.lock().await.app.clone_without_indexer();

    let indexer_httpd_context = indexer_httpd::context::Context::new(
        indexer_context,
        Arc::new(Mutex::new(app)),
        Arc::new(mock_client),
        indexer_path,
    );

    let dango_httpd_context =
        dango_httpd::context::Context::new(indexer_httpd_context.clone(), dango_context);

    dango_httpd::server::run_server("127.0.0.1", port, cors_allowed_origin, dango_httpd_context)
        .await
}

pub fn get_mock_socket_addr() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("failed to bind to random port")
        .local_addr()
        .expect("failed to get local address")
        .port()
}

pub async fn wait_for_server_ready(port: u16) -> anyhow::Result<()> {
    for attempt in 1..=30 {
        match TcpStream::connect(format!("127.0.0.1:{port}")).await {
            Ok(_) => {
                tracing::info!("Server ready on port {port} after {attempt} attempts");
                return Ok(());
            },
            Err(_) => {
                tracing::debug!("Attempt {attempt}: server not ready yet...");
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
        }
    }

    bail!("server failed to start on port {port} after 30 attempts")
}
