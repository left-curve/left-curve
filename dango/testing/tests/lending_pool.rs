use {
    dango_testing::setup_test,
    dango_types::lending_pool::{self, NAMESPACE},
    grug::{Addressable, Coins, Denom, Message, MsgTransfer, ResultExt, Uint128},
    std::{str::FromStr, sync::LazyLock},
    test_case::test_case,
};

static _ATOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uatom").unwrap());
static _OSMO: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uosmo").unwrap());
static USDC: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uusdc").unwrap());

#[test]
fn cant_transfer_to_lending_pool() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    suite
        .send_message(
            &mut accounts.relayer,
            Message::Transfer(MsgTransfer {
                to: contracts.lending_pool,
                coins: Coins::one(USDC.clone(), 123).unwrap(),
            }),
        )
        .should_fail_with_error("Can't send tokens to this contract");
}

#[test]
fn cant_deposit_from_margin_account() {
    todo!()
}

#[test_case(false; "no recipient arg")]
#[test_case(true; "with recipient arg")]
fn deposit_works(use_recipient: bool) -> anyhow::Result<()> {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    let recipient = if use_recipient {
        Some(accounts.owner.address())
    } else {
        None
    };

    let balance_before = suite.query_balance(&accounts.relayer, USDC.clone())?;

    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::Deposit { recipient },
            Coins::one(USDC.clone(), 123)?,
        )
        .should_succeed();

    // Ensure balance was deducted from depositor.
    suite
        .query_balance(&accounts.relayer, USDC.clone())
        .should_succeed_and_equal(balance_before - Uint128::new(123));

    // Ensure LP token was minted to recipient.
    let lp_denom = Denom::from_parts([NAMESPACE.to_string(), "lp".to_string(), USDC.to_string()])?;
    suite
        .query_balance(&recipient.unwrap_or(accounts.relayer.address()), lp_denom)
        .should_succeed_and_equal(Uint128::new(123));

    Ok(())
}

#[test_case(false; "no recipient arg")]
#[test_case(true; "with recipient arg")]
fn withdraw_works(use_recipient: bool) -> anyhow::Result<()> {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    let recipient = if use_recipient {
        Some(accounts.owner.address())
    } else {
        None
    };

    // First deposit
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::Deposit { recipient: None },
            Coins::one(USDC.clone(), 123)?,
        )
        .should_succeed();
    let lp_denom = Denom::from_parts([NAMESPACE.to_string(), "lp".to_string(), USDC.to_string()])?;
    suite
        .query_balance(&accounts.relayer.address(), lp_denom.clone())
        .should_succeed_and_equal(Uint128::new(123));

    let balance_before = suite.query_balance(
        &recipient.unwrap_or(accounts.relayer.address()),
        USDC.clone(),
    )?;

    // Now withdraw
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::Withdraw { recipient },
            Coins::one(lp_denom.clone(), 123)?,
        )
        .should_succeed();

    // Ensure LP token was burned from withdrawer.
    suite
        .query_balance(&accounts.relayer.address(), lp_denom)
        .should_succeed_and_equal(Uint128::new(0));

    // Ensure balance was added to recipient.
    suite
        .query_balance(
            &recipient.unwrap_or(accounts.relayer.address()),
            USDC.clone(),
        )
        .should_succeed_and_equal(balance_before + Uint128::new(123));

    Ok(())
}
