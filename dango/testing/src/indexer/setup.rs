use {
    grug_app::{Indexer, NaiveProposalPreparer},
    grug_db_memory::MemDb,
    grug_testing::{MockClient, TestAccounts, TestBuilder},
    grug_types::{BroadcastClientExt, Coins, Denom},
    grug_vm_rust::RustVm,
    indexer_hooked::HookedIndexer,
    indexer_httpd::{context::FullContext, traits::QueryApp},
    std::{str::FromStr, sync::Arc},
    tokio::sync::RwLock,
};

pub async fn create_hooked_indexer() -> (HookedIndexer, indexer_sql::Context, indexer_cache::Context)
{
    let sql_indexer = indexer_sql::IndexerBuilder::default()
        .with_memory_database()
        .build()
        .await
        .expect("Can't create indexer");

    let sql_indexer_context = sql_indexer.context.clone();

    let cache_indexer = indexer_cache::Cache::new_with_tempdir();
    let indexer_cache_context = cache_indexer.context.clone();

    let clickhouse_context = indexer_clickhouse::context::Context::new(
        "http://localhost:8123".to_string(),
        "default".to_string(),
        "default".to_string(),
        "default".to_string(),
    )
    .with_mock();
    let clickhouse_indexer = indexer_clickhouse::indexer::Indexer::new(clickhouse_context);

    let hooked_indexer = HookedIndexer::new(cache_indexer, sql_indexer, clickhouse_indexer);

    (hooked_indexer, sql_indexer_context, indexer_cache_context)
}

pub async fn create_block() -> anyhow::Result<(
    FullContext,
    Arc<MockClient<MemDb, RustVm, NaiveProposalPreparer, HookedIndexer>>,
    TestAccounts,
)> {
    create_blocks(1).await
}

pub async fn create_blocks(
    count: usize,
) -> anyhow::Result<(
    FullContext,
    Arc<MockClient<MemDb, RustVm, NaiveProposalPreparer, HookedIndexer>>,
    TestAccounts,
)> {
    let denom = Denom::from_str("ugrug")?;

    let (indexer, sql_indexer_context, indexer_cache_context) = create_hooked_indexer().await;

    let (suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000)?)
        .set_owner("owner")
        .build();

    let chain_id = suite.app.chain_id().await?;

    let suite = Arc::new(RwLock::new(suite));

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

    suite
        .read()
        .await
        .app
        .indexer
        .wait_for_finish()
        .await
        .expect("Can't wait for indexer to finish");

    let client = Arc::new(mock_client);

    let suite_guard = suite.read().await;
    let httpd_app = suite_guard.app.clone_without_indexer();
    let clickhouse_context = indexer_clickhouse::context::Context::new(
        "http://localhost:8123".to_string(),
        "default".to_string(),
        "default".to_string(),
        "default".to_string(),
    );
    let httpd_context = FullContext::new(
        indexer_cache_context,
        sql_indexer_context,
        clickhouse_context,
        Arc::new(httpd_app),
        client.clone(),
        None,
    );

    Ok((httpd_context, client, accounts))
}
