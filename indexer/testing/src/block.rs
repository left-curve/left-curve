use {
    grug_app::{Indexer, NaiveProposalPreparer},
    grug_db_memory::MemDb,
    grug_testing::{MockClient, TestAccounts, TestBuilder},
    grug_types::{BroadcastClientExt, Coins, Denom},
    grug_vm_rust::RustVm,
    httpd::traits::QueryApp,
    indexer_httpd::context::Context,
    indexer_sql::{hooks::NullHooks, non_blocking_indexer::NonBlockingIndexer},
    std::{str::FromStr, sync::Arc},
    tokio::sync::Mutex,
};

pub async fn create_block() -> anyhow::Result<(
    Context,
    Arc<MockClient<MemDb, RustVm, NaiveProposalPreparer, NonBlockingIndexer<NullHooks>>>,
    TestAccounts,
)> {
    create_blocks(1).await
}

pub async fn create_blocks(
    count: usize,
) -> anyhow::Result<(
    Context,
    Arc<MockClient<MemDb, RustVm, NaiveProposalPreparer, NonBlockingIndexer<NullHooks>>>,
    TestAccounts,
)> {
    let denom = Denom::from_str("ugrug")?;

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .with_database_max_connections(1)
        .with_keep_blocks(true)
        .build()?;

    let context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let (suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000)?)
        .set_owner("owner")
        .build();

    let chain_id = suite.app.chain_id().await?;

    let suite = Arc::new(Mutex::new(suite));

    let mock_client =
        MockClient::new_shared(suite.clone(), grug_testing::BlockCreation::OnBroadcast);

    let sender = accounts["sender"].address;

    for _ in 0..count {
        mock_client
            .send_message(
                &mut accounts["sender"],
                grug_types::Message::transfer(sender, Coins::one(denom.clone(), 2_000)?)?,
                grug_types::GasOption::Predefined { gas_limit: 2000 },
                &chain_id,
            )
            .await?;
    }

    suite.lock().await.app.indexer.wait_for_finish();

    let client = Arc::new(mock_client);

    let suite_guard = suite.lock().await;
    let httpd_app = suite_guard.app.clone_without_indexer();
    let httpd_context = Context::new(
        context,
        Arc::new(Mutex::new(httpd_app)),
        client.clone(),
        indexer_path,
    );

    Ok((httpd_context, client, accounts))
}
