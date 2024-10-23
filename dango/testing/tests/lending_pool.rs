use {
    dango_testing::setup_test,
    dango_types::{
        account::single,
        account_factory::AccountParams,
        lending_pool::{
            self, QueryDebtsRequest, QueryLiabilitiesRequest, QueryWhitelistedDenomsRequest,
            NAMESPACE,
        },
    },
    grug::{Addressable, Coin, Coins, Denom, HashExt, Message, MsgTransfer, ResultExt, Uint128},
    std::{str::FromStr, sync::LazyLock},
    test_case::test_case,
};

static ATOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("uatom").unwrap());
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
fn only_owner_can_whitelist_denoms() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    // Try to whitelist a denom from non-owner, should fail
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::WhitelistDenom(USDC.clone()),
            Coins::new(),
        )
        .should_fail_with_error("Only the owner can whitelist denoms");

    // Whitelist a denom from owner, should succeed
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::WhitelistDenom(USDC.clone()),
            Coins::new(),
        )
        .should_succeed();
}

#[test]
fn only_owner_can_delist_denoms() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    // Try to delist a denom from non-owner, should fail
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::DelistDenom(USDC.clone()),
            Coins::new(),
        )
        .should_fail_with_error("Only the owner can delist denoms");

    // Delist a denom from owner, should succeed
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::DelistDenom(USDC.clone()),
            Coins::new(),
        )
        .should_succeed();
}

#[test]
fn whitelist_denom_works() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    // Ensure USDC is already in the whitelist
    suite
        .query_wasm_smart(contracts.lending_pool, QueryWhitelistedDenomsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and_equal(vec![USDC.clone()]);

    // Try to whitelist a denom that is already in the whitelist, should fail
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::WhitelistDenom(USDC.clone()),
            Coins::new(),
        )
        .should_fail_with_error("Denom already whitelisted");

    // Whitelist ATOM from owner, should succeed
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::WhitelistDenom(ATOM.clone()),
            Coins::new(),
        )
        .should_succeed();

    // Ensure ATOM is now in the whitelist
    suite
        .query_wasm_smart(contracts.lending_pool, QueryWhitelistedDenomsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and_equal(vec![USDC.clone(), ATOM.clone()]);
}

#[test]
fn delist_denom_works() {
    let (mut suite, mut accounts, _codes, contracts) = setup_test();

    // Ensure USDC is already in the whitelist
    suite
        .query_wasm_smart(contracts.lending_pool, QueryWhitelistedDenomsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and_equal(vec![USDC.clone()]);

    // Delist denom not in the whitelist, should fail
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::DelistDenom(ATOM.clone()),
            Coins::new(),
        )
        .should_fail_with_error("Denom not whitelisted");

    // Delist USDC from owner, should succeed
    suite
        .execute(
            &mut accounts.owner,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::DelistDenom(USDC.clone()),
            Coins::new(),
        )
        .should_succeed();

    // Ensure USDC is no longer in the whitelist
    suite
        .query_wasm_smart(contracts.lending_pool, QueryWhitelistedDenomsRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and_equal(vec![]);
}

#[test]
fn cant_deposit_from_margin_account() -> anyhow::Result<()> {
    let (mut suite, mut accounts, codes, contracts) = setup_test();

    // Create a margin account.
    let mut margin_account = accounts.relayer.register_new_account(
        &mut suite,
        contracts.account_factory,
        codes.account_margin.to_bytes().hash256(),
        AccountParams::Margin(single::Params {
            owner: accounts.relayer.username.clone(),
        }),
        Coins::new(),
    )?;

    // Try to deposit from the margin account, should fail
    suite
        .execute(
            &mut margin_account,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::Deposit { recipient: None },
            Coins::new(),
        )
        .should_fail_with_error("Margin accounts can't deposit or withdraw");

    Ok(())
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

#[test]
fn borrowing_works() -> anyhow::Result<()> {
    let (mut suite, mut accounts, codes, contracts) = setup_test();

    // Create a margin account.
    let mut margin_account = accounts.relayer.register_new_account(
        &mut suite,
        contracts.account_factory,
        codes.account_margin.to_bytes().hash256(),
        AccountParams::Margin(single::Params {
            owner: accounts.relayer.username.clone(),
        }),
        Coins::new(),
    )?;

    // Try to borrow from the margin account, should succeed fail as no coins are deposited
    suite
        .execute(
            &mut margin_account,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::Borrow {
                coins: Coins::from(Coin::new(USDC.clone(), 100)?),
            },
            Coins::new(),
        )
        .should_fail_with_error("Not enough liquidity for uusdc. Max borrowable is 0");

    // Deposit some USDC
    suite
        .execute(
            &mut accounts.relayer,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::Deposit { recipient: None },
            Coins::one(USDC.clone(), 100)?,
        )
        .should_succeed();

    // Try to borrow again, should succeed
    suite
        .execute(
            &mut margin_account,
            contracts.lending_pool,
            &lending_pool::ExecuteMsg::Borrow {
                coins: Coins::from(Coin::new(USDC.clone(), 100)?),
            },
            Coins::new(),
        )
        .should_succeed();

    // Confirm the margin account has the borrowed coins
    suite
        .query_balance(&margin_account.address(), USDC.clone())
        .should_succeed_and_equal(Uint128::new(100));

    // Confirm that the lending pool has the liability
    suite
        .query_wasm_smart(
            contracts.lending_pool,
            QueryDebtsRequest(margin_account.address()),
        )
        .should_succeed_and_equal(Coins::from(Coin::new(USDC.clone(), 100)?));
    suite
        .query_wasm_smart(contracts.lending_pool, QueryLiabilitiesRequest {
            limit: None,
            start_after: None,
        })
        .should_succeed_and_equal(vec![(
            margin_account.address(),
            Coins::from(Coin::new(USDC.clone(), 100)?),
        )]);

    Ok(())
}
