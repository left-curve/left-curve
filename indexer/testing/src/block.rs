use {
    grug_app::NaiveProposalPreparer,
    grug_db_memory::MemDb,
    grug_testing::{MockClient, TestAccounts, TestBuilder},
    grug_types::{BroadcastClientExt, Coins, Denom},
    grug_vm_rust::RustVm,
    indexer_httpd::{context::Context, traits::QueryApp},
    indexer_sql::{hooks::NullHooks, non_blocking_indexer::NonBlockingIndexer},
    std::{str::FromStr, sync::Arc},
    tokio::sync::Mutex,
};

pub async fn create_block() -> anyhow::Result<(
    Context,
    Arc<MockClient<MemDb, RustVm, NaiveProposalPreparer, NonBlockingIndexer<NullHooks>>>,
    TestAccounts,
)> {
    let denom = Denom::from_str("ugrug")?;

    let indexer = indexer_sql::non_blocking_indexer::IndexerBuilder::default()
        .with_memory_database()
        .with_keep_blocks(true)
        .build()?;

    let context = indexer.context.clone();
    let indexer_path = indexer.indexer_path.clone();

    let (suite, mut accounts) = TestBuilder::new_with_indexer(indexer)
        .add_account("owner", Coins::new())
        .add_account("sender", Coins::one(denom.clone(), 30_000)?)
        .set_owner("owner")
        .build();

    let chain_id = suite.chain_id().await?;

    let suite = Arc::new(Mutex::new(suite));

    let mock_client =
        MockClient::new_shared(suite.clone(), grug_testing::BlockCreation::OnBroadcast);

    let sender = accounts["sender"].address;

    mock_client
        .send_message(
            &mut accounts["sender"],
            grug_types::Message::transfer(sender, Coins::one(denom.clone(), 2_000)?)?,
            grug_types::GasOption::Predefined { gas_limit: 2000 },
            &chain_id,
        )
        .await?;

    suite.lock().await.app.indexer.wait_for_finish();

    let client = Arc::new(mock_client);

    let httpd_context = Context::new(context, suite, client.clone(), indexer_path);

    Ok((httpd_context, client, accounts))
}
