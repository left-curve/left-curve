use {
    crate::{DEBTS, MARKETS},
    anyhow::{bail, ensure, Ok},
    dango_account_factory::ACCOUNTS,
    dango_types::{
        bank,
        lending::{ExecuteMsg, InstantiateMsg, Market, MarketUpdates, NAMESPACE, SUBNAMESPACE},
        DangoQuerier,
    },
    grug::{Coin, Coins, Denom, Message, MutableCtx, QuerierExt, Response, StorageQuerier},
    std::collections::BTreeMap,
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
        ExecuteMsg::Repay {} => repay(ctx),
    }
}

fn update_markets(
    ctx: MutableCtx,
    updates: BTreeMap<Denom, MarketUpdates>,
) -> anyhow::Result<Response> {
    // Ensure only chain owner can update markets denoms.
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "Only the owner can whitelist denoms"
    );

    for (denom, _updates) in updates {
        MARKETS.save(ctx.storage, &denom, &Market {})?;
    }

    Ok(Response::new())
}

fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    let bank = ctx.querier.query_bank()?;
    let mut msgs = vec![];

    for coin in ctx.funds {
        ensure!(MARKETS.has(ctx.storage, &coin.denom), "Invalid denom");

        let denom = coin.denom.prepend(&[&NAMESPACE, &SUBNAMESPACE])?;

        // TODO:
        // 1. compute LP token mint amount
        // 2. update `Market`
        let amount = coin.amount;

        msgs.push(Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.sender,
                denom,
                amount,
            },
            Coins::new(),
        )?);
    }

    Ok(Response::new().add_messages(msgs))
}

fn withdraw(ctx: MutableCtx) -> anyhow::Result<Response> {
    let bank = ctx.querier.query_bank()?;
    let mut msgs = vec![];
    let mut withdrawn = Coins::new();

    for coin in ctx.funds {
        let Some(underlying_denom) = coin.denom.strip(&[&NAMESPACE, &SUBNAMESPACE]) else {
            bail!("not a lending pool token: {}", coin.denom)
        };

        // TODO:
        // 1. compute withdraw amount
        // 2. update `Market`
        let underlying_amount = coin.amount;

        // Burn the LP tokens
        msgs.push(Message::execute(
            bank,
            &bank::ExecuteMsg::Burn {
                from: ctx.contract,
                denom: coin.denom,
                amount: coin.amount,
            },
            Coins::new(),
        )?);

        withdrawn.insert(Coin::new(underlying_denom, underlying_amount)?)?;
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_message(Message::transfer(ctx.sender, withdrawn)?))
}

fn borrow(ctx: MutableCtx, coins: Coins) -> anyhow::Result<Response> {
    let account_factory = ctx.querier.query_account_factory()?;

    // Ensure sender is a margin account.
    // An an optimization, use raw instead of smart query.
    ensure!(
        ctx.querier
            .query_wasm_path(account_factory, &ACCOUNTS.path(ctx.sender))?
            .params
            .is_margin(),
        "Only margin accounts can borrow and repay"
    );

    // Ensure the coins are whitelisted
    for coin in &coins {
        ensure!(
            MARKETS.has(ctx.storage, coin.denom),
            "Invalid denom. Only whitelisted denoms can be borrowed."
        );
    }

    // Update the sender's liabilities
    DEBTS.may_update(ctx.storage, ctx.sender, |maybe_debts| {
        let mut debts = maybe_debts.unwrap_or_default();

        debts.insert_many(coins.clone())?;

        Ok(debts)
    })?;

    // Transfer the coins to the caller
    Ok(Response::new().add_message(Message::transfer(ctx.sender, coins)?))
}

fn repay(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Ensure all sent coins are whitelisted
    for coin in &ctx.funds {
        ensure!(
            MARKETS.has(ctx.storage, coin.denom),
            "Invalid denom. Only whitelisted denoms can be repaid."
        );
    }

    let mut maybe_msg = None;

    // Update the sender's liabilities
    DEBTS.may_update(ctx.storage, ctx.sender, |maybe_debts| {
        let mut debts = maybe_debts.unwrap_or_default();

        // Deduct the sent coins from the account's debts, saturating at zero.
        let remainders = debts.saturating_deduct_many(ctx.funds)?;

        // Refund the remainders to the sender, if any.
        maybe_msg = Some(Message::transfer(ctx.sender, remainders)?);

        Ok(debts)
    })?;

    Ok(Response::new().may_add_message(maybe_msg))
}
