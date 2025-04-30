use {
    dango_genesis::build_rust_codes,
    dango_httpd::{graphql::build_schema, server::config_app},
    dango_proposal_preparer::ProposalPreparer,
    dango_testing::setup_suite_with_db_and_vm,
    grug_db_memory::MemDb,
    grug_testing::{MockClient, setup_tracing_subscriber},
    grug_vm_rust::RustVm,
    indexer_httpd::context::Context,
    std::sync::Arc,
    tokio::sync::Mutex,
    tracing::Level,
};

#[tokio::test(flavor = "multi_thread")]
async fn mock() {
    setup_tracing_subscriber(Level::INFO);

    let codes = build_rust_codes();

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .with_tmpdir()
        .with_hooks(dango_indexer_sql::hooks::Hooks)
        .build()
        .unwrap();

    let indexer_context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let db = MemDb::new();
    let vm = RustVm::new();

    let (suite, ..) = setup_suite_with_db_and_vm(
        db.clone(),
        vm.clone(),
        codes,
        ProposalPreparer::new_with_cache(),
        indexer,
    );

    let suite = Arc::new(Mutex::new(suite));

    let mock_client =
        MockClient::new_shared(suite.clone(), grug_testing::BlockCreation::OnBroadcast);

    let context = Context::new(
        indexer_context,
        Arc::new(suite),
        Arc::new(mock_client),
        indexer_path,
    );

    indexer_httpd::server::run_server("127.0.0.1", 8080, None, context, config_app, build_schema)
        .await
        .unwrap();
}
