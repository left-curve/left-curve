use {
    dango_genesis::build_rust_codes,
    dango_httpd::{graphql::build_schema, server::config_app},
    dango_indexer_sql::hooks::ContractAddrs,
    dango_proposal_preparer::ProposalPreparer,
    dango_testing::setup_suite_with_db_and_vm,
    grug_db_memory::MemDb,
    grug_testing::{MockClient, setup_tracing_subscriber},
    grug_types::Addr,
    grug_vm_rust::RustVm,
    indexer_httpd::context::Context,
    std::sync::Arc,
    tokio::sync::Mutex,
    tracing::Level,
};

pub use {dango_testing::TestOption, grug_testing::BlockCreation, indexer_httpd::error::Error};

pub async fn run(
    port: u16,
    block_creation: BlockCreation,
    cors_allowed_origin: Option<String>,
    test_opt: TestOption,
    keep_blocks: bool,
    database_url: Option<String>,
) -> Result<(), Error> {
    setup_tracing_subscriber(Level::INFO);

    let codes = build_rust_codes();

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
        .with_hooks(dango_indexer_sql::hooks::Hooks {
            contract_addrs: ContractAddrs {
                account_factory: Addr::mock(0),
            },
        })
        .build()?;

    let indexer_context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let db = MemDb::new();
    let vm = RustVm::new();

    let (suite, ..) = setup_suite_with_db_and_vm(
        db.clone(),
        vm.clone(),
        codes,
        ProposalPreparer::new(),
        indexer,
        test_opt,
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
