use {
    crate::utils::{setup_client_test, setup_client_test_with_port},
    dango_types::constants::usdc,
    futures::StreamExt,
    grug::{
        BroadcastClient, Coins, MOCK_CHAIN_ID, Message, NonEmpty, ResultExt, SearchTxClient, Signer,
    },
    indexer_client::{SubscribeBlock, WsClient, subscribe_block},
    std::time::Duration,
};

mod utils;

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

/// Verify that a single [`indexer_client::Session`] can host multiple
/// subscriptions over one WebSocket: each stream is routed independently and
/// dropping one does not disturb the others.
///
/// The mock server is configured with `BlockCreation::OnBroadcast`, so blocks
/// only appear when we broadcast a transaction — each broadcast drives one
/// round of deliveries to every active subscription.
#[tokio::test(flavor = "multi_thread")]
async fn session_multiplex() -> anyhow::Result<()> {
    let (http_client, mut accounts, port) = setup_client_test_with_port().await?;

    let ws = WsClient::new(format!("ws://localhost:{port}/graphql"))?;
    let session = ws.connect().await?;

    let mut blocks_a = session
        .subscribe::<SubscribeBlock>(subscribe_block::Variables {})
        .await?;
    let mut blocks_b = session
        .subscribe::<SubscribeBlock>(subscribe_block::Variables {})
        .await?;

    // Give the server a moment to register both subscriptions before producing
    // the first block — graphql-transport-ws has no "subscribed" ack.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let tx1 = accounts.user1.sign_transaction(
        NonEmpty::new_unchecked(vec![Message::transfer(
            accounts.user2.address.into_inner(),
            Coins::one(usdc::DENOM.clone(), 100)?,
        )?]),
        MOCK_CHAIN_ID,
        1_000_000,
    )?;
    http_client.broadcast_tx(tx1).await?;

    let b_a = tokio::time::timeout(Duration::from_secs(10), blocks_a.next())
        .await?
        .expect("blocks_a stream ended")
        .expect("blocks_a transport error");
    let b_b = tokio::time::timeout(Duration::from_secs(10), blocks_b.next())
        .await?
        .expect("blocks_b stream ended")
        .expect("blocks_b transport error");
    assert!(b_a.data.is_some(), "expected block payload on stream a");
    assert!(b_b.data.is_some(), "expected block payload on stream b");

    // Drop one stream; the other must keep flowing on the same connection.
    drop(blocks_b);

    let tx2 = accounts.user1.sign_transaction(
        NonEmpty::new_unchecked(vec![Message::transfer(
            accounts.user2.address.into_inner(),
            Coins::one(usdc::DENOM.clone(), 100)?,
        )?]),
        MOCK_CHAIN_ID,
        1_000_000,
    )?;
    http_client.broadcast_tx(tx2).await?;

    let b_a2 = tokio::time::timeout(Duration::from_secs(10), blocks_a.next())
        .await?
        .expect("blocks_a ended after sibling drop")
        .expect("blocks_a transport error after sibling drop");
    assert!(b_a2.data.is_some());

    Ok(())
}
