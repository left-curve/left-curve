use {
    crate::utils::setup_client_test,
    dango_types::constants::usdc,
    grug::{
        BroadcastClient, Coins, MOCK_CHAIN_ID, Message, NonEmpty, ResultExt, SearchTxClient, Signer,
    },
    std::time::Duration,
};

mod utils;

#[tokio::test]
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

    let tx_hash = res.tx_hash;

    client.search_tx(tx_hash).await?.outcome.should_succeed();

    Ok(())
}
