use {
    dango_genesis::GenesisCodes,
    dango_httpd::{graphql::build_schema, server::config_app},
    dango_proposal_preparer::ProposalPreparer,
    dango_testing::setup_suite_with_db_and_vm,
    grug_db_memory::MemDb,
    grug_testing::MockClient,
    grug_vm_rust::RustVm,
    indexer_httpd::context::Context,
    std::{net::TcpListener, sync::Arc, time::Duration},
    tokio::sync::Mutex,
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
    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default();

    let indexer = if let Some(url) = database_url {
        indexer.with_database_url(url)
    } else {
        indexer.with_memory_database()
    };

    let indexer = indexer
        .with_keep_blocks(keep_blocks)
        .with_sqlx_pubsub()
        .with_tmpdir()
        .with_hooks(dango_indexer_sql::hooks::Hooks)
        .build()?;

    let indexer_context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let (suite, ..) = setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        ProposalPreparer::new(),
        indexer,
        RustVm::genesis_codes(),
        test_opt,
        genesis_opt,
    );

    let suite = Arc::new(Mutex::new(suite));

    let mock_client = MockClient::new_shared(suite.clone(), block_creation);

    let context = Context::new(indexer_context, suite, Arc::new(mock_client), indexer_path);

    indexer_httpd::server::run_server(
        "127.0.0.1",
        port,
        cors_allowed_origin,
        context,
        config_app,
        build_schema,
    )
    .await
}

pub fn get_random_socket_addr() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to random port")
        .local_addr()
        .expect("Failed to get local address")
        .port()
}

pub async fn wait_for_server_ready(port: u16) -> anyhow::Result<()> {
    use tokio::net::TcpStream;

    for attempt in 1..=30 {
        // Try for 30 seconds
        match TcpStream::connect(format!("127.0.0.1:{port}")).await {
            Ok(_) => {
                tracing::info!("Server ready on port {port} after {attempt} attempts");
                return Ok(());
            },
            Err(_) => {
                tracing::debug!("Attempt {attempt}: Server not ready yet...");
                tokio::time::sleep(Duration::from_millis(50)).await;
            },
        }
    }

    anyhow::bail!("Server failed to start on port {port} after 30 attempts")
}
