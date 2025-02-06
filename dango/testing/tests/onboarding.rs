use {
    dango_bank::ORPHANED_TRANSFERS,
    dango_testing::{setup_test_naive, Factory, HyperlaneTestSuite, TestAccount},
    dango_types::{
        account::single,
        account_factory::{self, Account, AccountParams, NewUserSalt, Username},
        auth::Key,
        constants::USDC_DENOM,
    },
    grug::{
        btree_map, Addr, Addressable, ByteArray, Coin, Coins, HashExt, Json, Message, NonEmpty,
        QuerierExt, ResultExt, StdError, Tx, Uint128,
    },
    std::str::FromStr,
    test_case::test_case,
};

#[test]
fn user_onboarding() {
    let (suite, accounts, codes, contracts) = setup_test_naive();
    let (mut suite, _) = HyperlaneTestSuite::new_mocked(suite, accounts.owner);

    // Create a new key offchain; then, predict what its address would be.
    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
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
                username: user.username.clone(),
                key: user.pk,
            },
            Coins::new(),
        )
        .should_succeed();

    // The user's key should have been recorded in account factory.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeyRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(user.pk);

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
            msgs: NonEmpty::new_unchecked(vec![Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: user.username.clone(),
                    key: user.pk,
                },
                Coins::new(),
            )
            .unwrap()]),
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
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Send the register user transaction without making a deposit first.
    // Should fail during `CheckTx` with "data not found" error.
    let tx = Tx {
        sender: contracts.account_factory,
        gas_limit: 1_000_000,
        msgs: NonEmpty::new_unchecked(vec![Message::execute(
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                username: user.username.clone(),
                key: user.pk,
            },
            Coins::new(),
        )
        .unwrap()]),
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

/// A malicious block builder detects a register user transaction, inserts a new,
/// false transaction that substitutes the legitimate transaction's username or
/// key. Should fail because the derived deposit address won't match.
#[test_case(
    Some(Username::from_str("bad").unwrap()),
    None;
    "false username"
)]
#[test_case(
    None,
    Some(Key::Secp256k1(ByteArray::from([0; 33])));
    "false key"
)]
fn false_factory_tx(false_username: Option<Username>, false_key: Option<Key>) {
    let (mut suite, _, codes, contracts) = setup_test_naive();

    // User makes the deposit normally.
    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    let username = false_username.unwrap_or_else(|| user.username.clone());
    let key = false_key.unwrap_or(user.pk);

    // A malicious block builder sends a register user tx with falsified
    // username or key.
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
                    username: username.clone(),
                    key,
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
                    username: &username,
                    key,
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
