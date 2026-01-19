use {
    crate::utils::setup_client_test,
    dango_types::constants::usdc,
    grug::{
        BroadcastClient, Coins, MOCK_CHAIN_ID, Message, NonEmpty, ResultExt, SearchTxClient, Signer,
    },
};

mod utils;

/// Temporary test to measure server startup time in CI.
/// This test intentionally panics to show timing in CI output.
/// Remove after getting the timing info.
#[tokio::test(flavor = "multi_thread")]
async fn measure_server_startup_time() {
    let result = crate::utils::setup_client_test_with_timing().await.unwrap();
    panic!(
        "SERVER STARTUP TIMING: {} attempts, {:.2}ms - this panic is intentional to show timing in CI",
        result.server_ready.attempts,
        result.server_ready.elapsed_ms
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn broadcast() -> anyhow::Result<()> {
    let (client, mut accounts) = setup_client_test().await?;

    let tx = accounts.user1.sign_transaction(
        NonEmpty::new_unchecked(vec![Message::transfer(
            accounts.user2.address.into_inner(),
            Coins::one(usdc::DENOM.clone(), 100)?,
        )?]),
        MOCK_CHAIN_ID,
        1000000,
    )?;

    let res = client.broadcast_tx(tx).await?;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let tx_hash = res.tx_hash;

    client.search_tx(tx_hash).await?.outcome.should_succeed();

    Ok(())
}
