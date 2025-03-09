use {
    dango_testing::{MOCK_CHAIN_ID, TestAccounts, setup_test_naive},
    dango_types::{account::spot::QuerySeenNoncesRequest, constants::USDC_DENOM},
    grug::{
        Addressable, Coins, Duration, JsonSerExt, Message, NonEmpty, QuerierExt, ResultExt, Tx,
    },
    std::vec,
};

fn prepare_tx_with_nonce(accounts: &TestAccounts, nonce: u32, expiry: Option<Duration>) -> Tx {
    const GAS_LIMIT: u64 = 50_000_000;

    let msgs = NonEmpty::new_unchecked(vec![
        Message::transfer(
            accounts.user1.address(),
            Coins::one(USDC_DENOM.clone(), 123).unwrap(),
        )
        .unwrap(),
    ]);

    let (data, credential) = accounts
        .owner
        .sign_transaction_with_nonce(
            accounts.owner.address(),
            msgs.clone(),
            MOCK_CHAIN_ID,
            GAS_LIMIT,
            nonce,
            expiry,
        )
        .unwrap();

    Tx {
        gas_limit: GAS_LIMIT,
        msgs: msgs.clone(),
        sender: accounts.owner.address(),
        data: data.to_json_value().unwrap(),
        credential: credential.to_json_value().unwrap(),
    }
}

#[test]
fn tracked_nonces_works() {
    let (mut suite, mut accounts, ..) = setup_test_naive();

    for _ in 0..20 {
        suite
            .transfer(
                &mut accounts.owner,
                accounts.user1.address(),
                Coins::one(USDC_DENOM.clone(), 123).unwrap(),
            )
            .should_succeed();
    }

    // Query should return the next nonce.
    suite
        .query_wasm_smart(accounts.owner.address(), QuerySeenNoncesRequest {})
        .should_succeed_and(|seen_nonces| seen_nonces.last() == Some(&19));

    let tx = prepare_tx_with_nonce(&accounts, 20, None);
    suite.send_transaction(tx).should_succeed();

    // Transfer should fail because the nonce is already tracked.
    let tx = prepare_tx_with_nonce(&accounts, 9, None);
    suite.send_transaction(tx).should_fail();

    for i in 21..44 {
        if ![23, 25, 27, 29].contains(&i) {
            let tx = prepare_tx_with_nonce(&accounts, i, None);
            suite.send_transaction(tx).should_succeed();
        }
    }

    // A nonce in range and not used should still be valid.
    for i in [25, 27, 29] {
        let tx = prepare_tx_with_nonce(&accounts, i, None);
        suite.send_transaction(tx).should_succeed();
    }

    // A nonce not used but not in range shouldn't be valid.
    let tx = prepare_tx_with_nonce(&accounts, 23, None);
    suite.send_transaction(tx).should_fail();

    // A transaction with a valid nonce but expired shouldn't be valid.
    let tx = prepare_tx_with_nonce(&accounts, 45, Some(Duration::from_days(0)));
    suite.send_transaction(tx).should_fail();

    // Same tx but without expire should be valid.
    let tx = prepare_tx_with_nonce(&accounts, 45, None);
    suite.send_transaction(tx).should_succeed();
}
