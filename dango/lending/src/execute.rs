use {
    crate::{state::WHITELISTED_DENOMS, LIABILITIES},
    anyhow::{bail, ensure, Ok},
    dango_types::{
        account_factory::QueryAccountRequest,
        bank,
        config::ACCOUNT_FACTORY_KEY,
        lending::{ExecuteMsg, InstantiateMsg, NAMESPACE},
    },
    grug::{Addr, Coin, Coins, Denom, Inner, Message, MutableCtx, Part, Response},
    std::str::FromStr,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    // Store the whitelisted denoms.
    for denom in msg.whitelisted_denoms {
        WHITELISTED_DENOMS.insert(ctx.storage, denom)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn receive(_ctx: MutableCtx) -> anyhow::Result<Response> {
    // Reject all transfers.
    bail!("Can't send tokens to this contract. Use the `deposit` message instead.");
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::WhitelistDenom(denom) => whitelist_denom(ctx, denom),
        ExecuteMsg::DelistDenom(denom) => delist_denom(ctx, denom),
        ExecuteMsg::Deposit { recipient } => deposit(ctx, recipient),
        ExecuteMsg::Withdraw { recipient } => withdraw(ctx, recipient),
        ExecuteMsg::Borrow { coins } => borrow(ctx, coins),
    }
}

pub fn whitelist_denom(ctx: MutableCtx, denom: Denom) -> anyhow::Result<Response> {
    // Ensure only chain owner can whitelist denoms
    ensure!(
        ctx.sender == ctx.querier.query_config()?.owner,
        "Only the owner can whitelist denoms"
    );

    // Ensure the denom is not already in the whitelist
    ensure!(
        !WHITELISTED_DENOMS.has(ctx.storage, denom.clone()),
        "Denom already whitelisted"
    );

    // Insert the denom into the whitelist
    WHITELISTED_DENOMS.insert(ctx.storage, denom)?;

    Ok(Response::new())
}

pub fn delist_denom(ctx: MutableCtx, denom: Denom) -> anyhow::Result<Response> {
    // Ensure only chain owner can delist denoms
    ensure!(
        ctx.sender == ctx.querier.query_config()?.owner,
        "Only the owner can delist denoms"
    );

    // Ensure the denom is in the whitelist
    ensure!(
        WHITELISTED_DENOMS.has(ctx.storage, denom.clone()),
        "Denom not whitelisted"
    );

    // Remove the denom from the whitelist
    WHITELISTED_DENOMS.remove(ctx.storage, denom);

    Ok(Response::new())
}

/// Ensures that the sender's account is not a margin account.
fn ensure_sender_account_is_not_margin(ctx: &MutableCtx) -> anyhow::Result<()> {
    let account_factory: Addr = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;
    ensure!(
        !ctx.querier
            .query_wasm_smart(account_factory, QueryAccountRequest {
                address: ctx.sender,
            })?
            .params
            .is_margin(),
        "Margin accounts can't deposit or withdraw"
    );
    Ok(())
}

/// Ensures that the sender's account is a margin account.
fn ensure_sender_account_is_margin(ctx: &MutableCtx) -> anyhow::Result<()> {
    let account_factory: Addr = ctx.querier.query_app_config(ACCOUNT_FACTORY_KEY)?;
    ensure!(
        ctx.querier
            .query_wasm_smart(account_factory, QueryAccountRequest {
                address: ctx.sender,
            })?
            .params
            .is_margin(),
        "Only margin accounts can borrow and repay"
    );
    Ok(())
}

pub fn deposit(ctx: MutableCtx, recipient: Option<Addr>) -> anyhow::Result<Response> {
    // Ensure margin accounts can't deposit
    ensure_sender_account_is_not_margin(&ctx)?;

    // For each deposited denom, ensure it's whitelisted and mint LP tokens.
    let cfg = ctx.querier.query_config()?;
    let mut msgs = vec![];
    for coin in ctx.funds.into_iter() {
        ensure!(
            WHITELISTED_DENOMS.has(ctx.storage, coin.denom.clone()),
            "Invalid denom"
        );

        let mut parts = vec![Part::from_str(NAMESPACE)?, Part::from_str("lp")?];
        parts.extend_from_slice(coin.denom.inner());

        let lp_denom = Denom::from_parts(parts)?;

        msgs.push(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Mint {
                to: recipient.unwrap_or(ctx.sender),
                denom: lp_denom,
                amount: coin.amount,
            },
            Coins::new(),
        )?);
    }

    Ok(Response::new().add_messages(msgs))
}

pub fn withdraw(ctx: MutableCtx, recipient: Option<Addr>) -> anyhow::Result<Response> {
    // Ensure margin accounts can't withdraw
    ensure_sender_account_is_not_margin(&ctx)?;

    // Unwrap the recipient
    let recipient = recipient.unwrap_or(ctx.sender);

    // Ensure there are funds to withdraw
    ensure!(!ctx.funds.is_empty(), "No funds to withdraw");

    let cfg = ctx.querier.query_config()?;
    let mut msgs = vec![];
    let mut withdrawn = Coins::new();
    for coin in ctx.funds.into_iter() {
        // Ensure only LP tokens are sent
        let mut iter = coin.denom.inner().iter();

        ensure!(
            iter.next().map(|part| part.as_ref()) == Some(NAMESPACE),
            "namespace:{NAMESPACE} not found"
        );

        ensure!(
            iter.next().map(|part| part.as_ref()) == Some("lp"),
            "namespace: lp not found"
        );

        // let denom_parts = iter.map(|part| part.clone()).collect::<Vec<_>>();

        // Add msg to send the underlying tokens to the recipient
        let underlying_denom = Denom::from_parts(iter.cloned().collect::<Vec<_>>())?;
        let amount = coin.amount;
        withdrawn.insert(Coin::new(underlying_denom, amount)?)?;

        // Burn the LP tokens
        msgs.push(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Burn {
                from: ctx.contract,
                denom: coin.denom,
                amount,
            },
            Coins::new(),
        )?);
    }

    // Transfer the underlying tokens to the recipient
    msgs.push(Message::transfer(recipient, withdrawn)?);

    Ok(Response::new().add_messages(msgs))
}

pub fn borrow(ctx: MutableCtx, coins: Coins) -> anyhow::Result<Response> {
    // Ensure sender is a margin account
    ensure_sender_account_is_margin(&ctx)?;

    // Ensure the coins are whitelisted
    for coin in &coins {
        ensure!(
            WHITELISTED_DENOMS.has(ctx.storage, coin.denom.clone()),
            "Invalid denom. Only whitelisted denoms can be borrowed."
        );
    }

    // Update the sender's liabilities
    LIABILITIES.may_update(ctx.storage, ctx.sender, |debts| {
        let mut debts = debts.unwrap_or_default();
        debts.insert_many(coins.clone())?;
        Ok(debts)
    })?;

    // Transfer the coins to the caller
    Ok(Response::new().add_message(Message::transfer(ctx.sender, coins)?))
}