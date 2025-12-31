use {
    crate::setup::create_hooked_indexer,
    grug_app::{Indexer, NaiveProposalPreparer},
    grug_db_memory::MemDb,
    grug_httpd::traits::QueryApp,
    grug_testing::{MockClient, TestAccounts, TestBuilder},
    grug_types::{BroadcastClientExt, Coins, Denom},
    grug_vm_rust::RustVm,
    indexer_hooked::HookedIndexer,
    indexer_httpd::context::Context,
    std::{str::FromStr, sync::Arc},
    tokio::sync::Mutex,
};

pub async fn create_block() -> anyhow::Result<(
    Context,
    Arc<MockClient<MemDb, RustVm, NaiveProposalPreparer, HookedIndexer>>,
    TestAccounts,
)> {
    create_blocks(1).await
}

pub async fn create_blocks(
    count: usize,
) -> anyhow::Result<(
    Context,
    Arc<MockClient<MemDb, RustVm, NaiveProposalPreparer, HookedIndexer>>,
    TestAccounts,
)> {
    let denom = Denom::from_str("ugrug")?;

    let (indexer, sql_indexer_context, indexer_cache_context) = create_hooked_indexer();

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

    suite
        .lock()
        .await
        .app
        .indexer
        .wait_for_finish()
        .await
        .expect("Can't wait for indexer to finish");

    let client = Arc::new(mock_client);

    let suite_guard = suite.lock().await;
    let httpd_app = suite_guard.app.clone_without_indexer();
    let httpd_context = Context::new(
        indexer_cache_context,
        sql_indexer_context,
        Arc::new(httpd_app),
        client.clone(),
    );

    Ok((httpd_context, client, accounts))
}
