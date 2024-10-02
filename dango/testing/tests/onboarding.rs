use {
    dango_testing::{setup_test, Factory, TestAccount},
    dango_types::{
        account::single,
        account_factory::{self, Account, AccountParams, Username},
        auth::Key,
        mock_ibc_transfer,
    },
    grug::{
        btree_map, Addressable, ByteArray, Coins, Hash160, HashExt, Json, Message, ResultExt, Tx,
        Uint128,
    },
    std::str::FromStr,
    test_case::test_case,
};

#[test]
fn user_onboarding() -> anyhow::Result<()> {
    let (mut suite, mut accounts, codes, contracts) = setup_test()?;

    // Create a new key offchain; then, predict what its address would be.
    let user = TestAccount::new_random("user")?.predict_address(
        contracts.account_factory,
        codes.account_spot.to_bytes().hash256(),
        true,
    )?;

    // User makes an initial deposit. The relayer delivers the packet.
    // The funds is held inside the IBC transfer contract because the recipient
    // account doesn't exist yet.
    suite.execute(
        &mut accounts.relayer,
        contracts.ibc_transfer,
        &mock_ibc_transfer::ExecuteMsg::ReceiveTransfer {
            recipient: user.address(),
        },
        Coins::one("uusdc", 123)?,
    )?;

    // User uses account factory as sender to send an empty transaction.
    // Account factory should interpret this action as the user wishes to create
    // an account and claim the funds held in IBC transfer contract.
    suite
        .execute(
            &mut Factory::new(contracts.account_factory),
            contracts.account_factory,
            &account_factory::ExecuteMsg::RegisterUser {
                username: user.username.clone(),
                key: user.key,
                key_hash: user.key_hash,
            },
            Coins::new(),
        )
        .unwrap();

    // The user's key should have been recorded in account factory.
    suite
        .query_wasm_smart(
            contracts.account_factory,
            account_factory::QueryKeysByUserRequest {
                username: user.username.clone(),
            },
        )
        .should_succeed_and_equal(btree_map! { user.key_hash => user.key });

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
                // We have 3 genesis accounts (0, 1, 2) so this one should have
                // the index of 3.
                index: 3,
                params: AccountParams::Spot(single::Params { owner: user.username.clone() }),
            },
        });

    // User's account should have been created with the correct token balance.
    suite
        .query_balance(&user, "uusdc")
        .should_succeed_and_equal(Uint128::new(123));

    Ok(())
}

/// Attempt to register a username twice.
/// The transaction should fail `CheckTx` and be rejected from entering mempool.
#[test]
fn onboarding_existing_user() -> anyhow::Result<()> {
    let (mut suite, mut accounts, codes, contracts) = setup_test()?;

    // First, we onboard a user normally.
    let tx = {
        // Generate the key and derive address for the user.
        let user = TestAccount::new_random("user")?.predict_address(
            contracts.account_factory,
            codes.account_spot.to_bytes().hash256(),
            true,
        )?;

        // Make the initial deposit.
        suite.execute(
            &mut accounts.relayer,
            contracts.ibc_transfer,
            &mock_ibc_transfer::ExecuteMsg::ReceiveTransfer {
                recipient: user.address(),
            },
            Coins::one("uusdc", 123)?,
        )?;

        // Send the register user message with account factory.
        let tx = Tx {
            sender: contracts.account_factory,
            gas_limit: 1_000_000,
            msgs: vec![Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: user.username.clone(),
                    key: user.key,
                    key_hash: user.key_hash,
                },
                Coins::new(),
            )?],
            data: Json::Null,
            credential: Json::Null,
        };

        suite.send_transaction(tx.clone())?;

        tx
    };

    // Attempt to register the same username again, should fail.
    suite
        .check_tx(tx)?
        .result
        .should_fail_with_error("username `user` already exists");

    Ok(())
}

/// Attempt to register a user without first making a deposit.
/// The transaction should fail `CheckTx` and be rejected from entering mempool.
#[test]
fn onboarding_without_deposit() -> anyhow::Result<()> {
    let (suite, _, codes, contracts) = setup_test()?;

    let user = TestAccount::new_random("user")?.predict_address(
        contracts.account_factory,
        codes.account_spot.to_bytes().hash256(),
        true,
    )?;

    // Send the register user transaction without making a deposit first.
    // Should fail during `CheckTx` with "data not found" error.
    suite
        .check_tx(Tx {
            sender: contracts.account_factory,
            gas_limit: 1_000_000,
            msgs: vec![Message::execute(
                contracts.account_factory,
                &account_factory::ExecuteMsg::RegisterUser {
                    username: user.username.clone(),
                    key: user.key,
                    key_hash: user.key_hash,
                },
                Coins::new(),
            )?],
            data: Json::Null,
            credential: Json::Null,
        })?
        .result
        .should_fail_with_error("data not found!");

    Ok(())
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
    Some(Hash160::from_array([0; 20]));
    "false key hash"
)]
fn false_factory_tx(
    false_username: Option<Username>,
    false_key: Option<Key>,
    false_key_hash: Option<Hash160>,
) -> anyhow::Result<()> {
    let (mut suite, _, codes, contracts) = setup_test()?;

    // User makes the deposit normally.
    let user = TestAccount::new_random("user")?.predict_address(
        contracts.account_factory,
        codes.account_spot.to_bytes().hash256(),
        true,
    )?;

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
                    key: false_key.unwrap_or(user.key),
                    key_hash: false_key_hash.unwrap_or(user.key_hash),
                },
                Coins::new(),
            )?,
        )?
        .result
        .should_fail_with_error("data not found!");

    Ok(())
}
