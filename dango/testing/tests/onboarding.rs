use {
    dango_testing::{setup_test, Factory, TestAccount},
    dango_types::{
        account::single,
        account_factory::{self, Account, AccountParams, Username},
        auth::Key,
        ibc_transfer,
    },
    grug::{
        btree_map, Addressable, ByteArray, Coins, Hash160, HashExt, Json, Message, NonEmpty,
        ResultExt, Tx, Uint128,
    },
    std::str::FromStr,
    test_case::test_case,
};

#[test]
fn user_onboarding() {
    let (mut suite, mut accounts, codes, contracts) = setup_test();

    // Create a new key offchain; then, predict what its address would be.
    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // User makes an initial deposit. The relayer delivers the packet.
    // The funds is held inside the IBC transfer contract because the recipient
    // account doesn't exist yet.
    suite
        .execute(
            &mut accounts.relayer,
            contracts.ibc_transfer,
            &ibc_transfer::ExecuteMsg::ReceiveTransfer {
                recipient: user.address(),
            },
            Coins::one("uusdc", 123).unwrap(),
        )
        .should_succeed();

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in IBC transfer contract.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
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
                // We have 2 genesis accounts (0 owner, 1 relayer) so this one should have
                // the index of 2.
                index: 2,
                params: AccountParams::Spot(single::Params::new(user.username.clone() )),
            },
        });

    // User's account should have been created with the correct token balance.
    suite
        .query_balance(&user, "uusdc")
        .should_succeed_and_equal(Uint128::new(123));
}

/// Attempt to register a username twice.
/// The transaction should fail `CheckTx` and be rejected from entering mempool.
#[test]
fn onboarding_existing_user() {
    let (mut suite, mut accounts, codes, contracts) = setup_test();

    // First, we onboard a user normally.
    let tx = {
        // Generate the key and derive address for the user.
        let user = TestAccount::new_random("user").predict_address(
            contracts.account_factory,
            codes.account_spot.to_bytes().hash256(),
            true,
        );

        // Make the initial deposit.
        suite
            .execute(
                &mut accounts.relayer,
                contracts.ibc_transfer,
                &ibc_transfer::ExecuteMsg::ReceiveTransfer {
                    recipient: user.address(),
                },
                Coins::one("uusdc", 123).unwrap(),
            )
            .should_succeed();

        // Send the register user message with account factory.
        let tx = Tx {
            sender: contracts.account_factory,
            gas_limit: 1_000_000,
            msgs: NonEmpty::new_unchecked(vec![Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: user.username.clone(),
                    key: user.first_key(),
                    key_hash: user.first_key_hash(),
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
    let (suite, _, codes, contracts) = setup_test();

    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // Send the register user transaction without making a deposit first.
    // Should fail during `CheckTx` with "data not found" error.
    suite
        .check_tx(Tx {
            sender: contracts.account_factory,
            gas_limit: 1_000_000,
            msgs: NonEmpty::new_unchecked(vec![Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: user.username.clone(),
                    key: user.first_key(),
                    key_hash: user.first_key_hash(),
                },
                Coins::new(),
            )
            .unwrap()]),
            data: Json::null(),
            credential: Json::null(),
        })
        .should_fail_with_error("data not found!");
}

/// A malicious block builder detects a register user transaction, inserts a new,
/// false transaction that substitutes the legitimate transaction's username,
/// key, or key hash. Should fail because the derived deposit address won't match.
#[test_case(
    Some(Username::from_str("bad").unwrap()),
    None,
    None;
    "false username"
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
    Some(Hash160::from_inner([0; 20]));
    "false key hash"
)]
fn false_factory_tx(
    false_username: Option<Username>,
    false_key: Option<Key>,
    false_key_hash: Option<Hash160>,
) {
    let (mut suite, _, codes, contracts) = setup_test();

    // User makes the deposit normally.
    let user = TestAccount::new_random("user").predict_address(
        contracts.account_factory,
        codes.account_spot.to_bytes().hash256(),
        true,
    );

    // A malicious block builder sends a register user tx with falsified
    // username, key, or key hash.
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
                    username: false_username.unwrap_or_else(|| user.username.clone()),
                    key: false_key.unwrap_or(user.first_key()),
                    key_hash: false_key_hash.unwrap_or(user.first_key_hash()),
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error("data not found!");
}
