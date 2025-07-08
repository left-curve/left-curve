use {
    // assertor::*,
    dango_testing::{HyperlaneTestSuite, create_user_and_account, setup_test_with_indexer},
    grug::setup_tracing_subscriber,
    grug_app::Indexer,
    tracing::Level,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn index_candles() -> anyhow::Result<()> {
    setup_tracing_subscriber(Level::INFO);

    let (suite, mut accounts, codes, contracts, validator_sets, ..) =
        setup_test_with_indexer().await;
    let mut suite = HyperlaneTestSuite::new(suite, validator_sets, &contracts);

    let _user = create_user_and_account(&mut suite, &mut accounts, &contracts, &codes, "user");

    suite.app.indexer.wait_for_finish()?;

    Ok(())
}
