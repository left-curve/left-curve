use {
    dango_testing::setup_test,
    dango_types::token_factory::{ExecuteMsg, NAMESPACE},
    grug::{Addressable, Coins, Denom, Message, ResultExt, Uint128},
    std::{str::FromStr, sync::LazyLock},
};

static SUBDENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("umars").unwrap());

#[test]
fn token_factory() {
    let (mut suite, mut accounts, _, contracts) = setup_test().unwrap();

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
        .unwrap()
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
        .unwrap()
        .should_fail_with_error("incorrect denom creation fee!");

    // Attempt to create a denom for another username. Should fail.
    suite
        .send_message(
            &mut accounts.owner,
            Message::execute(
                contracts.token_factory,
                &ExecuteMsg::Create {
                    subdenom: SUBDENOM.clone(),
                    username: Some(accounts.fee_recipient.username.clone()), // wrong!
                    admin: None,
                },
                Coins::one("uusdc", 10_000_000).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
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
            Coins::one("uusdc", 10_000_000).unwrap(),
        )
        .unwrap();

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
                Coins::one("uusdc", 10_000_000).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
        .should_fail_with_error("already exists");

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
                    to: accounts.fee_recipient.address(),
                    amount: Uint128::new(12_345),
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .unwrap()
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
                    to: accounts.fee_recipient.address(),
                    amount: Uint128::new(12_345),
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .unwrap()
        .should_fail_with_error("data not found");

    // Correctly mint a token.
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &ExecuteMsg::Mint {
                denom: denom.clone(),
                to: accounts.fee_recipient.address(),
                amount: Uint128::new(12_345),
            },
            Coins::new(),
        )
        .unwrap();

    // The recipient's balance should have been updated.
    suite
        .query_balance(&accounts.fee_recipient, denom.clone())
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
                    from: accounts.fee_recipient.address(),
                    amount: Uint128::new(88_888),
                },
                Coins::new(),
            )
            .unwrap(),
        )
        .unwrap()
        .should_fail_with_error("subtraction overflow");

    // Properly burn the token.
    suite
        .execute(
            &mut accounts.owner,
            contracts.token_factory,
            &ExecuteMsg::Burn {
                denom: denom.clone(),
                from: accounts.fee_recipient.address(),
                amount: Uint128::new(2_345),
            },
            Coins::new(),
        )
        .unwrap();

    // The recipient's balance should have been updated.
    suite
        .query_balance(&accounts.fee_recipient, denom)
        .should_succeed_and_equal(Uint128::new(10_000));
}
