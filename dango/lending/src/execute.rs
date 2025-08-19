use {
    crate::{DEBTS, MARKETS, core},
    anyhow::ensure,
    dango_account_factory::ACCOUNTS,
    dango_types::{
        DangoQuerier, bank,
        lending::{Borrowed, ExecuteMsg, InstantiateMsg, InterestRateModel, Market, Repaid},
    },
    grug::{
        Coins, Denom, Inner, Message, MutableCtx, NonEmpty, Order, QuerierExt, Response,
        StorageQuerier,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (denom, interest_rate_model) in msg.markets {
        MARKETS.save(
            ctx.storage,
            &denom,
            &Market::new(&denom, interest_rate_model)?,
        )?;
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
        ExecuteMsg::ClaimPendingProtocolFees {} => claim_pending_protocol_fees(ctx),
    }
}

fn update_markets(
    ctx: MutableCtx,
    updates: BTreeMap<Denom, InterestRateModel>,
) -> anyhow::Result<Response> {
    // Ensure only chain owner can update markets denoms.
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "only the owner can whitelist denoms"
    );

    for (denom, new_interest_rate_model) in updates {
        MARKETS.may_update(ctx.storage, &denom, |maybe_market| -> anyhow::Result<_> {
            if let Some(market) = maybe_market {
                // Update indexes first, so that interests accumulated up to this
                // point are accounted for. Then, set the new interest rate model.
                let market = core::update_indices(market, ctx.querier, ctx.block.timestamp)?;
                Ok(market.set_interest_rate_model(new_interest_rate_model))
            } else {
                Ok(Market::new(&denom, new_interest_rate_model)?)
            }
        })?;
    }

    Ok(Response::new())
}

fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Immutably update markets and compute the amount of LP tokens to mint.
    let (lp_tokens, markets) =
        core::deposit(ctx.storage, ctx.querier, ctx.block.timestamp, ctx.funds)?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Mint the LP tokens to the sender.
    Ok(Response::new().add_message({
        let bank = ctx.querier.query_bank()?;
        Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.sender,
                coins: lp_tokens,
            },
            Coins::new(),
        )?
    }))
}

fn withdraw(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Immutably update markets and compute the amount of underlying coins to withdraw
    let (withdrawn, markets) = core::withdraw(
        ctx.storage,
        ctx.querier,
        ctx.block.timestamp,
        ctx.funds.clone(),
    )?;

    // Save the updated markets
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // 1. Burn the LP tokens.
    // 2. Transfer the underlying coins to the caller.
    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    coins: ctx.funds,
                },
                Coins::new(),
            )?
        })
        .may_add_message(Message::transfer(ctx.sender, withdrawn)?))
}

fn borrow(ctx: MutableCtx, coins: NonEmpty<Coins>) -> anyhow::Result<Response> {
    let account_factory = ctx.querier.query_account_factory()?;

    // Ensure sender is a margin account.
    // An an optimization, use raw instead of smart query.
    ensure!(
        ctx.querier
            .query_wasm_path(account_factory, &ACCOUNTS.path(ctx.sender))?
            .params
            .is_margin(),
        "only margin accounts can borrow and repay"
    );

    let (debts, markets) = core::borrow(
        ctx.storage,
        ctx.querier,
        ctx.block.timestamp,
        ctx.sender,
        coins.inner(),
    )?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Save the updated debts
    DEBTS.save(ctx.storage, ctx.sender, &debts)?;

    // Transfer the coins to the caller
    Ok(Response::new()
        .may_add_message(Message::transfer(ctx.sender, coins.inner().clone())?)
        .add_event(Borrowed {
            user: ctx.sender,
            borrowed: coins.into_inner(),
        })?)
}

fn repay(ctx: MutableCtx) -> anyhow::Result<Response> {
    let (scaled_debts, markets, refunds) = core::repay(
        ctx.storage,
        ctx.querier,
        ctx.block.timestamp,
        ctx.sender,
        &ctx.funds,
    )?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Save the updated debts.
    if scaled_debts.is_empty() {
        DEBTS.remove(ctx.storage, ctx.sender);
    } else {
        DEBTS.save(ctx.storage, ctx.sender, &scaled_debts)?;
    };

    Ok(Response::new()
        .may_add_message(Message::transfer(ctx.sender, refunds.clone())?)
        .add_event(Repaid {
            user: ctx.sender,
            repaid: ctx.funds,
            refunds,
            remaining_scaled_debts: scaled_debts,
        })?)
}

fn claim_pending_protocol_fees(ctx: MutableCtx) -> anyhow::Result<Response> {
    let bank = ctx.querier.query_bank()?;
    let owner = ctx.querier.query_owner()?;
    let mut coins_to_mint = Coins::new();

    let markets = MARKETS
        .range(ctx.storage, None, None, Order::Ascending)
        .map(|res| -> anyhow::Result<_> {
            let (denom, market) = res?;
            let market = core::update_indices(market, ctx.querier, ctx.block.timestamp)?;

            coins_to_mint.insert((
                market.supply_lp_denom.clone(),
                market.pending_protocol_fee_scaled,
            ))?;

            Ok((denom, market.reset_pending_protocol_fee()))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    Ok(Response::new().add_message(Message::execute(
        bank,
        &bank::ExecuteMsg::Mint {
            to: owner,
            coins: coins_to_mint,
        },
        Coins::new(),
    )?))
}
