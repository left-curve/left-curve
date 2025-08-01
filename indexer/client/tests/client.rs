use {
    crate::utils::setup_client_test,
    dango_types::constants::usdc,
    grug::{
        BroadcastClient, Coins, MOCK_CHAIN_ID, Message, NonEmpty, ResultExt, SearchTxClient, Signer,
    },
};

mod utils;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn broadcast() -> anyhow::Result<()> {
    let (client, mut accounts) = setup_client_test().await?;

    let tx = accounts.user1.sign_transaction(
        NonEmpty::new_unchecked(vec![
            Message::transfer(
                accounts.user2.address.into_inner(),
                Coins::one(usdc::DENOM.clone(), 100)?,
            )?
            .unwrap(), // safe to unwrap because we know the coins is non-empty
        ]),
        MOCK_CHAIN_ID,
        1000000,
    )?;

    let res = client.broadcast_tx(tx).await?;

    let tx_hash = res.tx_hash;

    client.search_tx(tx_hash).await?.outcome.should_succeed();

    Ok(())
}
