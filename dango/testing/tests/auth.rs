use {
    dango_auth::MAX_NONCE_INCREASE,
    dango_testing::setup_test_naive,
    dango_types::constants::dango,
    grug::{Addressable, JsonSerExt, Message, NonEmpty, ResultExt, Tx, coins},
};

/// An account's first ever transaction typically have a nonce of 0. However, we
/// don't want to enforce the first nonce to be exactly zero, because if a user
/// submits their 1st and 2nd txs (with nonces 0 and 1, respectively) simultaneously,
/// and the 2nd tx happens to land at the node first, then it will fail (because
/// it comes with nonce 1, not zero).
///
/// Instead, we enforce that the first ever nonce can't be bigger than `MAX_NONCE_INCREASE`,
/// which is set to 100.
#[test]
fn first_nonce_too_big() {
    let (suite, accounts, ..) = setup_test_naive();

    // User attempts to send their first tx with nonce 101. Should fail.
    {
        let msgs = NonEmpty::new_unchecked(vec![
            Message::transfer(
                accounts.user2.address(),
                coins! { dango::DENOM.clone() => 100 },
            )
            .unwrap(),
        ]);

        let (metadata, credential) = accounts
            .user1
            .sign_transaction_with_nonce(
                accounts.user1.address(),
                msgs.clone(),
                &suite.chain_id,
                100_000,
                101, // illegal nonce
                None,
            )
            .unwrap();

        let tx = Tx {
            sender: accounts.user1.address(),
            gas_limit: 100_000,
            msgs,
            data: metadata.to_json_value().unwrap(),
            credential: credential.to_json_value().unwrap(),
        };

        suite.check_tx(tx).should_fail_with_error(format!(
            "first nonce is too big: {} >= MAX_NONCE_INCREASE ({})",
            101, MAX_NONCE_INCREASE
        ));
    }
}
