use {
    anyhow::bail,
    dango_genesis::{Codes, Contracts, GenesisCodes},
    dango_proposal_preparer::ProposalPreparer,
    dango_testing::{TestAccounts, setup_suite_with_db_and_vm},
    grug_app::NullIndexer,
    grug_db_memory::MemDb,
    grug_testing::MockClient,
    grug_vm_rust::{ContractWrapper, RustVm},
    httpd::{context::Context as HttpdContext, graphql::build_schema, server::config_app},
    hyperlane_testing::MockValidatorSets,
    std::{net::TcpListener, sync::Arc, time::Duration},
    tokio::{net::TcpStream, sync::Mutex},
};
pub use {
    dango_genesis::GenesisOption,
    dango_testing::{BridgeOp, Preset, TestOption},
    grug_testing::BlockCreation,
    httpd::error::Error,
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
    _keep_blocks: bool,
    _database_url: Option<String>,
    callback: C,
) -> Result<(), Error>
where
    C: FnOnce(TestAccounts, Codes<ContractWrapper>, Contracts, MockValidatorSets) + Send + Sync,
{
    let (suite, test, codes, contracts, mock_validator_sets) = setup_suite_with_db_and_vm(
        MemDb::new(),
        RustVm::new(),
        ProposalPreparer::new(),
        NullIndexer,
        RustVm::genesis_codes(),
        test_opt,
        genesis_opt,
    );

    callback(test, codes, contracts, mock_validator_sets);

    let suite = Arc::new(Mutex::new(suite));

    let _mock_client = MockClient::new_shared(suite.clone(), block_creation);

    let app = suite.lock().await.app.clone();
    let context = HttpdContext::new(Arc::new(Mutex::new(app)));

    httpd::server::run_server(
        "127.0.0.1",
        port,
        cors_allowed_origin,
        context,
        config_app,
        build_schema,
    )
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
