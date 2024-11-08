use {
    dango_testing::{setup_test, TOKEN_FACTORY_CREATION_FEE},
    dango_types::{
        bank::{Metadata, QueryMetadataRequest},
        token_factory::{Config, ExecuteMsg, NAMESPACE},
    },
    grug::{Addressable, Coins, Denom, Message, NonEmpty, ResultExt, Uint128},
    std::{str::FromStr, sync::LazyLock},
};

static SUBDENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("umars").unwrap());

#[test]
fn token_factory() {
    let (mut suite, mut accounts, _, contracts) = setup_test();

    let owner_username = accounts.owner.username.clone();

    // ---------------------------- Token creation -----------------------------

    // Attempt to create a denom without sending fee. Should fail.
    // For simplicity, we just use the "owner" account throughout this test.
    suite
        .send_message(
            &mut accounts.owner,
            Message::execute(
                contracts.token_factory,
                &ExecuteMsg::Create {
                    username: Some(owner_username.clone()),
                    subdenom: SUBDENOM.clone(),
                    admin: None,
                },
                Coins::new(), // wrong!
            )
            .unwrap(),
        )
        .should_fail_with_error("invalid payment: expecting 1 coins, found 0");

    // Attempt to create a denom with more fee than needed. Should fail.
    suite
        .send_message(
            &mut accounts.owner,
            Message::execute(
                contracts.token_factory,
                &ExecuteMsg::Create {
                    subdenom: SUBDENOM.clone(),
                    username: Some(owner_username.clone()),
                    admin: None,
                },
                Coins::one("uusdc", 20_000_000).unwrap(), // wrong!
            )
            .unwrap(),
        )
        .should_fail_with_error("incorrect denom creation fee!");

    // Attempt to create a denom for another username. Should fail.
    suite
        .send_message(
            &mut accounts.owner,
            Message::execute(
                contracts.token_factory,
                &ExecuteMsg::Create {
                    subdenom: SUBDENOM.clone(),
                    username: Some(accounts.relayer.username.clone()), // wrong!
                    admin: None,
                },
                Coins::from(TOKEN_FACTORY_CREATION_FEE.clone()),
            )
            .unwrap(),
        )
        .should_fail_with_error("isn't associated with username");

    // Finally, correctly create a denom.
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &ExecuteMsg::Create {
                subdenom: SUBDENOM.clone(),
                username: Some(owner_username.clone()),
                admin: None,
            },
            Coins::from(TOKEN_FACTORY_CREATION_FEE.clone()),
        )
        .should_succeed();

    // Attempt to create the same denom again. Should fail.
    suite
        .send_message(
            &mut accounts.owner,
            Message::execute(
                contracts.token_factory,
                &ExecuteMsg::Create {
                    subdenom: SUBDENOM.clone(),
                    username: Some(owner_username.clone()),
                    admin: None,
                },
                Coins::from(TOKEN_FACTORY_CREATION_FEE.clone()),
            )
            .unwrap(),
        )
        .should_fail_with_error("already exists");

    // Taxman should have received the token creation fee.
    suite
        .query_balance(&contracts.taxman, "uusdc")
        .should_succeed_and_equal(Uint128::new(10_000_000));

    // ----------------------------- Token minting -----------------------------

    // The full denom that should have been just created.
    let denom = Denom::from_parts([
        NAMESPACE.to_string(),
        accounts.owner.username.to_string(),
        SUBDENOM.to_string(),
    ])
    .unwrap();

    // Attempt to mint another user's token. Should fail.
    suite
        .send_message(
            &mut accounts.relayer, // wrong!
            Message::execute(
                contracts.token_factory,
                &ExecuteMsg::Mint {
                    denom: denom.clone(),
                    to: accounts.owner.address(),
                    amount: Uint128::new(12_345),
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error("sender isn't the admin of denom");

    // Attempt to mint a non-existent token. Should fail.
    suite
        .send_message(
            &mut accounts.owner,
            Message::execute(
                contracts.token_factory,
                &ExecuteMsg::Mint {
                    denom: Denom::from_parts([
                        NAMESPACE.to_string(),
                        owner_username.to_string(),
                        "uosmo".to_string(), // wrong!
                    ])
                    .unwrap(),
                    to: accounts.relayer.address(),
                    amount: Uint128::new(12_345),
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error("data not found");

    // Correctly mint a token.
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &ExecuteMsg::Mint {
                denom: denom.clone(),
                to: accounts.relayer.address(),
                amount: Uint128::new(12_345),
            },
            Coins::new(),
        )
        .should_succeed();

    // The recipient's balance should have been updated.
    suite
        .query_balance(&accounts.relayer, denom.clone())
        .should_succeed_and_equal(Uint128::new(12_345));

    // ----------------------------- Token burning -----------------------------

    // Attempt to burn more than the balance. Should fail.
    suite
        .send_message(
            &mut accounts.owner,
            Message::execute(
                contracts.token_factory,
                &ExecuteMsg::Burn {
                    denom: denom.clone(),
                    from: accounts.relayer.address(),
                    amount: Uint128::new(88_888),
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .should_fail_with_error("subtraction overflow");

    // Properly burn the token.
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &ExecuteMsg::Burn {
                denom: denom.clone(),
                from: accounts.relayer.address(),
                amount: Uint128::new(2_345),
            },
            Coins::new(),
        )
        .should_succeed();

    // The recipient's balance should have been updated.
    suite
        .query_balance(&accounts.relayer, denom)
        .should_succeed_and_equal(Uint128::new(10_000));

    // ------------------------ Zero denom creation fee ------------------------

    // Set denom creation fee to zero.
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &ExecuteMsg::Configure {
                new_cfg: Config {
                    token_creation_fee: None,
                },
            },
            Coins::new(),
        )
        .should_succeed();

    // Attempt to create a denom without sending fee. Should succeed.
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &ExecuteMsg::Create {
                username: Some(owner_username.clone()),
                subdenom: Denom::from_str("hello").unwrap(),
                admin: None,
            },
            Coins::new(),
        )
        .should_succeed();
}

#[test]
fn metadata() {
    let (mut suite, mut account, _, contracts) = setup_test();

    let subdenom = Denom::from_str("foo").unwrap();

    // Register a new denom
    suite
        .execute(
            &mut account.relayer,
            contracts.token_factory,
            &ExecuteMsg::Create {
                subdenom: subdenom.clone(),
                username: None,
                admin: None,
            },
            Coins::from(TOKEN_FACTORY_CREATION_FEE.clone()),
        )
        .should_succeed();

    let denom = Denom::from_str(&format!("factory/{}/foo", account.relayer.address())).unwrap();

    let metadata = Metadata {
        name: NonEmpty::new_unchecked("Foo".to_string()),
        symbol: NonEmpty::new_unchecked("FO".to_string()),
        description: "A test token".to_string(),
        decimals: 6,
    };

    // Try set metadata on token_factory from non admin
    suite
        .execute(
            &mut account.owner,
            contracts.token_factory,
            &ExecuteMsg::SetMetadata {
                denom: denom.clone(),
                metadata: metadata.clone(),
            },
            Coins::default(),
        )
        .should_fail_with_error("sender isn't the admin of denom");

    // Try set metadata on bank from the admin of the denom
    suite
        .execute(
            &mut account.owner,
            contracts.bank,
            &ExecuteMsg::SetMetadata {
                denom: denom.clone(),
                metadata: metadata.clone(),
            },
            Coins::default(),
        )
        .should_fail_with_error("sender does not own the namespace");

    // Ok set metadata on token_factory from the admin of the denom
    suite
        .execute(
            &mut account.relayer,
            contracts.token_factory,
            &ExecuteMsg::SetMetadata {
                denom: denom.clone(),
                metadata: metadata.clone(),
            },
            Coins::default(),
        )
        .should_succeed();

    // Query metadata
    let res = suite
        .query_wasm_smart(contracts.bank, QueryMetadataRequest { denom })
        .should_succeed();

    assert_eq!(res, metadata);
}
