use {
    grug_math::Uint128,
    grug_mock_account::Credential,
    grug_testing::TestBuilder,
    grug_types::{Coins, Duration, JsonDeExt, Message, NonEmpty, ResultExt, Timestamp, Tx},
};

#[tokio::test]
async fn check_tx_and_finalize() {
    let (mut suite, mut accounts) = TestBuilder::new()
        .add_account("rhaki", Coins::one("uatom", 100).unwrap())
        .add_account("larry", Coins::new())
        .add_account("owner", Coins::new())
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .set_owner("owner")
        .build();

    let transfer_msg =
        Message::transfer(accounts["larry"].address, Coins::one("uatom", 10).unwrap()).unwrap();

    // Create a tx to set sequence to 1.
    suite
        .send_message(&mut accounts["rhaki"], transfer_msg.clone())
        .await
        .should_succeed();

    // Create a tx with sequence 0, 1, 2, 4.
    let txs: Vec<Tx> = [0, 1, 2, 4]
        .into_iter()
        .filter_map(|sequence| {
            // Sign the tx
            let tx = accounts["rhaki"]
                .sign_transaction_with_sequence(
                    NonEmpty::new_unchecked(vec![transfer_msg.clone()]),
                    &suite.chain_id,
                    sequence,
                    0,
                )
                .ok()?;

            // Check the tx and if the result is ok, return the tx.
            //
            // Note: there are two layers of results here:
            // - `check_tx` must succeed, meaning the chain itself doesn't
            //   run into any error, so we `unwrap`.
            // - The `Outcome::result` returned by `checked_tx` may fail,
            //   so we gracefully handle it with `?`.
            suite.check_tx(tx.clone()).result.ok()?;

            Some(tx)
        })
        .collect();

    // The tx with sequence 0 should fails check_tx.
    assert_eq!(txs.len(), 3);
    assert_eq!(
        txs[0]
            .credential
            .clone()
            .deserialize_json::<Credential>()
            .unwrap()
            .sequence,
        1
    );
    assert_eq!(
        txs[1]
            .credential
            .clone()
            .deserialize_json::<Credential>()
            .unwrap()
            .sequence,
        2
    );
    assert_eq!(
        txs[2]
            .credential
            .clone()
            .deserialize_json::<Credential>()
            .unwrap()
            .sequence,
        4
    );

    // Create a block with the txs.
    // The tx with sequence 1 should succeed.
    // The tx with sequence 2 should succeed.
    // The tx with sequence 4 should fail.
    let result = suite.make_block(txs).await.block_outcome;

    result.tx_outcomes[0].clone().should_succeed();
    result.tx_outcomes[1].clone().should_succeed();
    result.tx_outcomes[2].clone().should_fail();

    suite
        .query_balance(&accounts["rhaki"], "uatom")
        .should_succeed_and_equal(Uint128::new(70));
    suite
        .query_balance(&accounts["larry"], "uatom")
        .should_succeed_and_equal(Uint128::new(30));

    // Try create a block with a tx with sequence = 3
    let tx = accounts["rhaki"]
        .sign_transaction_with_sequence(
            NonEmpty::new_unchecked(vec![transfer_msg]),
            &suite.chain_id,
            3,
            0,
        )
        .unwrap();

    suite.make_block(vec![tx]).await.block_outcome.tx_outcomes[0]
        .clone()
        .should_succeed();

    suite
        .query_balance(&accounts["rhaki"], "uatom")
        .should_succeed_and_equal(Uint128::new(60));
    suite
        .query_balance(&accounts["larry"], "uatom")
        .should_succeed_and_equal(Uint128::new(40));
}
