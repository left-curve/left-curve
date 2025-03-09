use {
    dango_bank::ORPHANED_TRANSFERS,
    dango_testing::{Factory, HyperlaneTestSuite, TestAccount, setup_test_naive},
    dango_types::{
        account::single,
        account_factory::{self, Account, AccountParams, NewUserSalt},
        auth::Key,
        constants::USDC_DENOM,
    },
    grug::{
        Addr, Addressable, ByteArray, Coin, Coins, Hash256, HashExt, Json, JsonSerExt, Message,
        NonEmpty, Op, QuerierExt, ResultExt, StdError, Tx, Uint128, btree_map,
    },
    test_case::test_case,
};

#[test]
fn user_onboarding() {
    let (suite, accounts, codes, contracts) = setup_test_naive();
    let (mut suite, _) = HyperlaneTestSuite::new_mocked(suite, accounts.owner);

    // Create a new key offchain; then, predict what its address would be.
    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        0,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Make the initial deposit.
    suite.hyperlane().recieve_transfer(
        user.address(),
        Coin::new(USDC_DENOM.clone(), 10_000_000).unwrap(),
    );

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in IBC transfer contract.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                secret: 0,
                username: user.username.clone(),
                key: user.first_key(),
                key_hash: user.first_key_hash(),
            },
            Coins::new(),
        )
        .should_succeed();

    // The user's key should have been recorded in account factory.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! { user.first_key_hash() => user.first_key() });

    // The user's account info should have been recorded in account factory.
    // Note: a user's first ever account is always a spot account.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryAccountsByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! {
            user.address() => Account {
                // We have 10 genesis accounts (owner + users 1-9), indexed from
                // zero, so this one should have the index of 10.
                index: 10,
                params: AccountParams::Spot(single::Params::new(user.username.clone() )),
            },
        });

    // User's account should have been created with the correct token balance.
    suite
        .query_balance(&user, USDC_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));
}

/// Attempt to register a username twice.
/// The transaction should fail `CheckTx` and be rejected from entering mempool.
#[test]
fn onboarding_existing_user() {
    let (suite, accounts, codes, contracts) = setup_test_naive();
    let (mut suite, _) = HyperlaneTestSuite::new_mocked(suite, accounts.owner);

    // First, we onboard a user normally.
    let tx = {
        // Generate the key and derive address for the user.
        let user = TestAccount::new_random("user").predict_address(
            contracts.account_factory,
            10,
            codes.account_spot.to_bytes().hash256(),
            true,
        );

        // Make the initial deposit.
        suite.hyperlane().recieve_transfer(
            user.address(),
            Coin::new(USDC_DENOM.clone(), 10_000_000).unwrap(),
        );

        // Send the register user message with account factory.
        let tx = Tx {
            sender: contracts.account_factory,
            gas_limit: 1_000_000,
            msgs: NonEmpty::new_unchecked(vec![
                Message::execute(
                    contracts.account_factory,
                    &account_factory::ExecuteMsg::RegisterUser {
                        username: user.username.clone(),
                        key: user.first_key(),
                        key_hash: user.first_key_hash(),
                        secret: 10,
                    },
                    Coins::new(),
                )
                .unwrap(),
            ]),
            data: Json::null(),
            credential: Json::null(),
        };

        suite.send_transaction(tx.clone()).should_succeed();

        tx
    };

    // Attempt to register the same username again, should fail.
    suite
        .check_tx(tx)
        .should_fail_with_error("username `user` already exists");
}

/// Attempt to register a user without first making a deposit.
/// The transaction should fail `CheckTx` and be rejected from entering mempool.
#[test]
fn onboarding_without_deposit() {
    let (suite, accounts, codes, contracts) = setup_test_naive();
    let (mut suite, _) = HyperlaneTestSuite::new_mocked(suite, accounts.owner);

    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        3,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Send the register user transaction without making a deposit first.
    // Should fail during `CheckTx` with "data not found" error.
    let tx = Tx {
        sender: contracts.account_factory,
        gas_limit: 1_000_000,
        msgs: NonEmpty::new_unchecked(vec![
            Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    secret: 3,
                    username: user.username.clone(),
                    key: user.first_key(),
                    key_hash: user.first_key_hash(),
                },
                Coins::new(),
            )
            .unwrap(),
        ]),
        data: Json::null(),
        credential: Json::null(),
    };

    suite
        .check_tx(tx.clone())
        .should_fail_with_error(StdError::data_not_found::<Coins>(
            ORPHANED_TRANSFERS
                .path((contracts.warp, user.address()))
                .storage_key(),
        ));

    // Make a deposit but not enough.
    suite.hyperlane().recieve_transfer(
        user.address(),
        Coin::new(USDC_DENOM.clone(), 7_000_000).unwrap(),
    );

    // Try again, should fail.
    suite
        .check_tx(tx.clone())
        .should_fail_with_error("minumum deposit not satisfied");

    // Make a deposit of the minimum amount.
    suite.hyperlane().recieve_transfer(
        user.address(),
        Coin::new(USDC_DENOM.clone(), 3_000_000).unwrap(),
    );

    // Try again, should succeed.
    suite.check_tx(tx).should_succeed();
}

#[test]
fn update_key() {
    let (suite, accounts, codes, contracts) = setup_test_naive();
    let (mut suite, _) = HyperlaneTestSuite::new_mocked(suite, accounts.owner);

    // Create a new key offchain; then, predict what its address would be.
    let mut user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        0,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Make the initial deposit.
    suite.hyperlane().recieve_transfer(
        user.address(),
        Coin::new(USDC_DENOM.clone(), 10_000_000).unwrap(),
    );

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in IBC transfer contract.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                secret: 0,
                username: user.username.clone(),
                key: user.first_key(),
                key_hash: user.first_key_hash(),
            },
            Coins::new(),
        )
        .should_succeed();

    // Try to delete last key, should fail.
    let first_key_hash = user.first_key_hash();
    suite
        .execute(
            &mut user,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Delete,
                key_hash: first_key_hash,
            },
            Coins::new(),
        )
        .should_fail_with_error(format!(
            "can't delete the last key associated with username `{}`",
            user.username
        ));

    // Query keys should return only one key.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! { user.first_key_hash() => user.first_key() });

    // Add a new key to the user's account.
    let (_, pk) = TestAccount::new_key_pair();
    let key_hash = pk.to_json_vec().unwrap().hash256();
    suite
        .execute(
            &mut user,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Insert(pk),
                key_hash,
            },
            Coins::new(),
        )
        .should_succeed();

    // Query keys should return two keys.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! {
            user.first_key_hash() => user.first_key(),
            key_hash => pk,
        });

    // Delete the first key should be possible since there is another key.
    suite
        .execute(
            &mut user,
            contracts.account_factory,
            &account_factory::ExecuteMsg::UpdateKey {
                key: Op::Delete,
                key_hash: first_key_hash,
            },
            Coins::new(),
        )
        .should_succeed();

    // Query keys should return only one key.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! { key_hash => pk });
}

/// A malicious block builder detects a register user transaction, inserts a new,
/// false transaction that substitutes the legitimate transaction's secret,
/// key, or key hash. Should fail because the derived deposit address won't match.
#[test_case(
    Some(5),
    None,
    None;
    "false secret"
)]
#[test_case(
    None,
    Some(Key::Secp256k1(ByteArray::from([0; 33]))),
    None;
    "false key"
)]
#[test_case(
    None,
    None,
    Some(Hash256::from_inner([0; 32]));
    "false key hash"
)]
fn false_factory_tx(
    false_secret: Option<u32>,
    false_key: Option<Key>,
    false_key_hash: Option<Hash256>,
) {
    let (mut suite, _, codes, contracts) = setup_test_naive();

    // User makes the deposit normally.
    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        2,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    let secret = false_secret.unwrap_or(2);
    let key = false_key.unwrap_or(user.first_key());
    let key_hash = false_key_hash.unwrap_or(user.first_key_hash());

    // A malicious block builder sends a register user tx with falsified
    // secret, key, or key hash.
    //
    // Should fail with "data not found" error, because it be different deposit
    // address for which no deposit is found.
    //
    // We test with `FinalizedBlock` here instead of with `CheckTx`, because a
    // malicious block builder can bypass mempool check.
    suite
        .send_message(
            &mut Factory::new(contracts.account_factory),
            Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: user.username.clone(),
                    secret,
                    key,
                    key_hash,
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error({
            let false_address = Addr::derive(
                contracts.account_factory,
                codes.account_spot.to_bytes().hash256(),
                &NewUserSalt {
                    secret,
                    key,
                    key_hash,
                }
                .into_bytes(),
            );

            StdError::data_not_found::<Coins>(
                ORPHANED_TRANSFERS
                    .path((contracts.warp, false_address))
                    .storage_key(),
            )
        });
}
