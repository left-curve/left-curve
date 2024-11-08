use {
    crate::{DEBTS, MARKETS},
    anyhow::{anyhow, bail, ensure, Ok},
    dango_account_factory::ACCOUNTS,
    dango_types::{
        account_factory::Account,
        bank,
        config::AppConfig,
        lending::{ExecuteMsg, InstantiateMsg, Market, MarketUpdates, NAMESPACE, SUBNAMESPACE},
    },
    grug::{BorshDeExt, Coin, Coins, Denom, Message, MutableCtx, Response},
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

pub fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

    let mut msgs = vec![];

    for coin in ctx.funds {
        ensure!(MARKETS.has(ctx.storage, &coin.denom), "Invalid denom");

        let denom = coin.denom.prepend(&[&NAMESPACE, &SUBNAMESPACE])?;

        // TODO:
        // 1. compute LP token mint amount
        // 2. update `Market`
        let amount = coin.amount;

        msgs.push(Message::execute(
            cfg.bank,
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

pub fn withdraw(ctx: MutableCtx) -> anyhow::Result<Response> {
    let cfg = ctx.querier.query_config()?;

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
            cfg.bank,
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

pub fn borrow(ctx: MutableCtx, coins: Coins) -> anyhow::Result<Response> {
    let app_cfg: AppConfig = ctx.querier.query_app_config()?;

    // Ensure sender is a margin account.
    // An an optimization, use raw instead of smart query.
    ensure!(
        ctx.querier
            .query_wasm_raw(app_cfg.addresses.account_factory, ACCOUNTS.path(ctx.sender))?
            .ok_or_else(|| anyhow!(
                "borrower {} is not registered in account factory",
                ctx.sender
            ))?
            .deserialize_borsh::<Account>()?
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
