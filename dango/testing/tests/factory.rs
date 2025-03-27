use {
    dango_bank::ORPHANED_TRANSFERS,
    dango_testing::{Factory, HyperlaneTestSuite, TestAccount, setup_test_naive},
    dango_types::{
        account::single,
        account_factory::{
            self, Account, AccountParams, QueryAccountRequest, RegisterUserData, Username,
        },
        bank,
        constants::USDC_DENOM,
    },
    grug::{
        Addressable, Coin, Coins, HashExt, Json, JsonSerExt, Message, NonEmpty, Op, QuerierExt,
        ResultExt, StdError, Tx, Uint128, VerificationError, btree_map, coins,
    },
    std::str::FromStr,
};

#[test]
fn user_onboarding() {
    let (suite, accounts, codes, contracts) = setup_test_naive();
    let (mut suite, _) = HyperlaneTestSuite::new_mocked(suite, accounts.owner);

    let chain_id = suite.chain_id.clone();

    // Create a new key offchain; then, predict what its address would be.
    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        0,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Make the initial deposit.
    suite.hyperlane().receive_transfer(
        user.address(),
        Coin::new(USDC_DENOM.clone(), 10_000_000).unwrap(),
    );

    // The transfer should be an orphaned transfer. The bank contract should be
    // holding the 10 USDC.
    suite
        .query_balance(&contracts.bank, USDC_DENOM.clone())
        .should_succeed_and_equal(Uint128::new(10_000_000));

    // The orphaned transfer should have been recorded.
    suite
        .query_wasm_smart(contracts.bank, bank::QueryOrphanedTransferRequest {
            sender: contracts.warp,
            recipient: user.address(),
        })
        .should_succeed_and_equal(coins! { USDC_DENOM.clone() => 10_000_000 });

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in orphaned transfer in bank.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                username: user.username.clone(),
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                seed: 0,
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        username: user.username.clone(),
                        chain_id: chain_id.clone(),
                    })
                    .unwrap(),
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

    let chain_id = suite.chain_id.clone();

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
        suite.hyperlane().receive_transfer(
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
                        seed: 10,
                        signature: user
                            .sign_arbitrary(RegisterUserData {
                                username: user.username.clone(),
                                chain_id: chain_id.clone(),
                            })
                            .unwrap(),
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

    let chain_id = suite.chain_id.clone();

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
                    username: user.username.clone(),
                    key: user.first_key(),
                    key_hash: user.first_key_hash(),
                    seed: 3,
                    signature: user
                        .sign_arbitrary(RegisterUserData {
                            username: user.username.clone(),
                            chain_id: chain_id.clone(),
                        })
                        .unwrap(),
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
    suite.hyperlane().receive_transfer(
        user.address(),
        Coin::new(USDC_DENOM.clone(), 7_000_000).unwrap(),
    );

    // Try again, should fail.
    suite
        .check_tx(tx.clone())
        .should_fail_with_error("minumum deposit not satisfied");

    // Make a deposit of the minimum amount.
    suite.hyperlane().receive_transfer(
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

    let chain_id = suite.chain_id.clone();

    // Create a new key offchain; then, predict what its address would be.
    let mut user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        0,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Make the initial deposit.
    suite.hyperlane().receive_transfer(
        user.address(),
        Coin::new(USDC_DENOM.clone(), 10_000_000).unwrap(),
    );

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in orphaned transfer in bank.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                username: user.username.clone(),
                key: user.first_key(),
                key_hash: user.first_key_hash(),
                seed: 0,
                signature: user
                    .sign_arbitrary(RegisterUserData {
                        username: user.username.clone(),
                        chain_id: chain_id.clone(),
                    })
                    .unwrap(),
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

/// A malicious block builder detects a register user transaction, could try to,
/// false transaction that substitutes the legitimate transaction's username.
/// Should fail because the signature won't be valid.
#[test]
fn malicious_register_user() {
    let (suite, accounts, codes, contracts) = setup_test_naive();
    let (mut suite, _) = HyperlaneTestSuite::new_mocked(suite, accounts.owner);
    let chain_id = suite.chain_id.clone();

    // ------------------------------ User setup -------------------------------

    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        2,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // User makes deposit.
    suite.hyperlane().receive_transfer(
        user.address(),
        Coin::new(USDC_DENOM.clone(), 10_000_000).unwrap(),
    );

    // The frontend instructs the user to sign this.
    let user_signature = user
        .sign_arbitrary(RegisterUserData {
            username: user.username.clone(),
            chain_id: chain_id.clone(),
        })
        .unwrap();

    // ---------------------------- Attacker setup -----------------------------

    // The attacker attempts to register this username using the user's deposit.
    let false_username = Username::from_str("random_username").unwrap();

    let attacker = TestAccount::new_random("attacker").predict_address(
        contracts.account_factory,
        2,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    let false_signature = attacker
        .sign_arbitrary(RegisterUserData {
            username: false_username.clone(),
            chain_id: chain_id.clone(),
        })
        .unwrap();

    // --------------------------- Attack commences ----------------------------

    // The attacker first tries to substitute the `username` field in the
    // `ExecuteMsg::RegisterUser`, but does not change the user signature.
    suite
        .send_message(
            &mut Factory::new(contracts.account_factory),
            Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: false_username.clone(),  // attacker falsified this
                    signature: user_signature.clone(), // attacker keeps this unchaged
                    key: user.first_key(),
                    key_hash: user.first_key_hash(),
                    seed: 2,
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error(VerificationError::Unauthentic);

    // The attacker can also try falsify a signature. Since the attacker doesn't
    // know the user's private key, he signs with a different private key.
    suite
        .send_message(
            &mut Factory::new(contracts.account_factory),
            Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: false_username.clone(),   // attacker falsified this
                    signature: false_signature.clone(), // this time, attacker also falsifies this
                    key: user.first_key(),              // but attacker keeps this unchanged
                    key_hash: user.first_key_hash(),
                    seed: 2,
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error(VerificationError::Unauthentic);

    // What if attacker also changes the `key` in the instantiate message?
    // Signature verification should pass, but the derived deposit address would
    // be different. Instantiation of the spot account should fail as no deposit
    // is found.
    suite
        .send_message(
            &mut Factory::new(contracts.account_factory),
            Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: false_username.clone(), // attacker falsified this
                    signature: false_signature,       // attacker falsifies this
                    key: attacker.first_key(),        // this time, attacker also falsifies these
                    key_hash: attacker.first_key_hash(),
                    seed: 2,
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error(
            // The derived deposit address would be the attacker's address.
            // No orphaned transfer from the Warp contract to the attacker
            // is found.
            StdError::data_not_found::<Coins>(
                dango_bank::ORPHANED_TRANSFERS
                    .path((contracts.warp, attacker.address()))
                    .storage_key(),
            ),
        );

    // Finally, user properly registers the username.
    suite
        .send_message(
            &mut Factory::new(contracts.account_factory),
            Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: user.username.clone(),
                    signature: user_signature,
                    key: user.first_key(),
                    key_hash: user.first_key_hash(),
                    seed: 2,
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_succeed();

    // User's account should have been created under the correct username.
    suite
        .query_wasm_smart(contracts.account_factory, QueryAccountRequest {
            address: user.address(),
        })
        .should_succeed_and(|account| account.params.clone().as_spot().owner == user.username);
}
