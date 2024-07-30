use {
    grug_account::Credential,
    grug_testing::TestBuilder,
    grug_types::{from_json_value, Coins, Duration, Message, Timestamp, Tx, Uint128},
};

#[test]
fn check_tx_and_finalize() -> anyhow::Result<()> {
    let (mut suite, accounts) = TestBuilder::new()
        .add_account("rhaki", [("uatom", 100_u128)])?
        .add_account("larry", Coins::new())?
        .add_account("owner", Coins::new())?
        .set_genesis_time(Timestamp::from_nanos(0))
        .set_block_time(Duration::from_seconds(1))
        .set_owner("owner")?
        .build()?;

    let transfer_msg = Message::transfer(
        accounts["larry"].address.clone(),
        Coins::one("uatom", Uint128::new(10).into()),
    )?;

    // Create a tx to set sequence to 1.
    suite.send_message(&accounts["rhaki"], transfer_msg.clone())?;

    // Create a tx with sequence 0, 1, 2, 4.

    let info = suite.query_info().should_succeed();

    let txs: Vec<Tx> = [0, 1, 2, 4]
        .into_iter()
        .filter_map(|sequence| {
            (|| -> anyhow::Result<_> {
                // Sign the tx
                let tx = accounts["rhaki"].sign_transaction(
                    vec![transfer_msg.clone()],
                    0,
                    &info.chain_id,
                    sequence,
                )?;
                // Check the tx and if the result is ok, return the tx.
                suite.check_tx(tx.clone())?.result.into_std_result()?;

                Ok(tx)
            })()
            .ok()
        })
        .collect();

    // The tx with sequence 0 should fails check_tx.
    assert_eq!(txs.len(), 3);
    assert_eq!(
        from_json_value::<Credential>(txs[0].credential.clone())?.sequence,
        1
    );
    assert_eq!(
        from_json_value::<Credential>(txs[1].credential.clone())?.sequence,
        2
    );
    assert_eq!(
        from_json_value::<Credential>(txs[2].credential.clone())?.sequence,
        4
    );

    // Create a block with the txs.
    // The tx with sequence 1 should succeed.
    // The tx with sequence 2 should succeed.
    // The tx with sequence 4 should fail.
    let result = suite.make_block(txs)?;

    result.tx_outcomes[0].result.clone().should_succeed();
    result.tx_outcomes[1].result.clone().should_succeed();
    result.tx_outcomes[2].result.clone().should_fail();

    assert_eq!(
        suite
            .query_balance(&accounts["rhaki"], "uatom")
            .should_succeed(),
        70_u128.into()
    );
    assert_eq!(
        suite
            .query_balance(&accounts["larry"], "uatom")
            .should_succeed(),
        30_u128.into()
    );

    // Try create a block with a tx with sequence = 3
    let tx =
        accounts["rhaki"].sign_transaction(vec![transfer_msg.clone()], 0, &info.chain_id, 3)?;

    suite.make_block(vec![tx])?.tx_outcomes[0]
        .result
        .clone()
        .should_succeed();

    assert_eq!(
        suite
            .query_balance(&accounts["rhaki"], "uatom")
            .should_succeed(),
        60_u128.into()
    );
    assert_eq!(
        suite
            .query_balance(&accounts["larry"], "uatom")
            .should_succeed(),
        40_u128.into()
    );

    Ok(())
}
