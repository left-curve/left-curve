use {
    crate::{DEBTS, MARKETS},
    anyhow::{ensure, Ok},
    dango_types::{
        account_factory::QueryAccountRequest,
        bank,
        config::ACCOUNT_FACTORY_KEY,
        lending::{ExecuteMsg, InstantiateMsg, Market, MarketUpdates, NAMESPACE},
    },
    grug::{Addr, Coin, Coins, Denom, Inner, Message, MutableCtx, Part, Response},
    std::{collections::BTreeMap, str::FromStr},
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (denom, _updates) in msg.markets {
        MARKETS.save(ctx.storage, &denom, &Market {})?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateMarkets(updates) => update_markets(ctx, updates),
        ExecuteMsg::Deposit {} => deposit(ctx),
        ExecuteMsg::Withdraw {} => withdraw(ctx),
        ExecuteMsg::Borrow(coins) => borrow(ctx, coins),
    }
}

fn update_markets(
    ctx: MutableCtx,
    updates: BTreeMap<Denom, MarketUpdates>,
) -> anyhow::Result<Response> {
    // Ensure only chain owner can update markets denoms.
    ensure!(
        ctx.sender == ctx.querier.query_config()?.owner,
        "Only the owner can whitelist denoms"
    );

    for (denom, _updates) in updates {
        MARKETS.save(ctx.storage, &denom, &Market {})?;
    }

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

pub fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Ensure margin accounts can't deposit
    ensure_sender_account_is_not_margin(&ctx)?;

    // For each deposited denom, ensure it's whitelisted and mint LP tokens.
    let cfg = ctx.querier.query_config()?;
    let mut msgs = vec![];
    for coin in ctx.funds {
        ensure!(MARKETS.has(ctx.storage, &coin.denom), "Invalid denom");

        let mut parts = vec![Part::from_str(NAMESPACE)?, Part::from_str("lp")?];
        parts.extend_from_slice(coin.denom.inner());

        let lp_denom = Denom::from_parts(parts)?;

        msgs.push(Message::execute(
            cfg.bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.sender,
                denom: lp_denom,
                amount: coin.amount,
            },
            Coins::new(),
        )?);
    }

    Ok(Response::new().add_messages(msgs))
}

pub fn withdraw(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Ensure margin accounts can't withdraw
    ensure_sender_account_is_not_margin(&ctx)?;

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
    msgs.push(Message::transfer(ctx.sender, withdrawn)?);

    Ok(Response::new().add_messages(msgs))
}

pub fn borrow(ctx: MutableCtx, coins: Coins) -> anyhow::Result<Response> {
    // Ensure sender is a margin account
    ensure_sender_account_is_margin(&ctx)?;

    // Ensure the coins are whitelisted
    for coin in &coins {
        ensure!(
            MARKETS.has(ctx.storage, coin.denom),
            "Invalid denom. Only whitelisted denoms can be borrowed."
        );
    }

    // Update the sender's liabilities
    DEBTS.may_update(ctx.storage, ctx.sender, |debts| {
        let mut debts = debts.unwrap_or_default();
        debts.insert_many(coins.clone())?;
        Ok(debts)
    })?;

    // Transfer the coins to the caller
    Ok(Response::new().add_message(Message::transfer(ctx.sender, coins)?))
}
