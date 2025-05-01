use {
    crate::{ASSETS, DEBTS, MARKETS, core},
    anyhow::ensure,
    dango_account_factory::ACCOUNTS,
    dango_types::{
        DangoQuerier,
        lending::{Borrowed, ExecuteMsg, InstantiateMsg, InterestRateModel, Market, Repaid},
    },
    grug::{
        Coins, Denom, Inner, Message, MutableCtx, Number, Order, QuerierExt, Response, StdResult,
        StorageQuerier,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (denom, interest_rate_model) in msg.markets {
        MARKETS.save(ctx.storage, &denom, &Market::new(interest_rate_model)?)?;
    }

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::UpdateMarkets(updates) => update_markets(ctx, updates),
        ExecuteMsg::Deposit {} => deposit(ctx),
        ExecuteMsg::Withdraw(coins) => withdraw(ctx, coins.into_inner()),
        ExecuteMsg::Borrow(coins) => borrow(ctx, coins.into_inner()),
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
                let market = core::update_indices(market, ctx.block.timestamp)?;
                Ok(market.set_interest_rate_model(new_interest_rate_model))
            } else {
                Ok(Market::new(new_interest_rate_model)?)
            }
        })?;
    }

    Ok(Response::new())
}

fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    let (assets, markets) =
        core::deposit(ctx.storage, ctx.block.timestamp, ctx.sender, &ctx.funds)?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Save the user's updated assets.
    ASSETS.save(ctx.storage, ctx.sender, &assets)?;

    Ok(Response::new())
    // TODO: add `Deposited` event
}

fn withdraw(ctx: MutableCtx, coins: Coins) -> anyhow::Result<Response> {
    let (assets, markets) = core::withdraw(ctx.storage, ctx.block.timestamp, ctx.sender, &coins)?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Save the user's updated assets.
    if assets.is_empty() {
        ASSETS.remove(ctx.storage, ctx.sender);
    } else {
        ASSETS.save(ctx.storage, ctx.sender, &assets)?;
    };

    Ok(Response::new().add_message(Message::transfer(ctx.sender, coins)?))
    // TODO: add `Withdrawn` event
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
        "only margin accounts can borrow and repay"
    );

    let (debts, markets) = core::borrow(ctx.storage, ctx.block.timestamp, ctx.sender, &coins)?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Save the user's updated debts.
    DEBTS.save(ctx.storage, ctx.sender, &debts)?;

    // Transfer the coins to the caller
    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, coins.clone())?)
        .add_event(Borrowed {
            user: ctx.sender,
            borrowed: coins,
        })?)
}

fn repay(ctx: MutableCtx) -> anyhow::Result<Response> {
    let (debts, markets, refunds) =
        core::repay(ctx.storage, ctx.block.timestamp, ctx.sender, &ctx.funds)?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Save the updated debts.
    if debts.is_empty() {
        DEBTS.remove(ctx.storage, ctx.sender);
    } else {
        DEBTS.save(ctx.storage, ctx.sender, &debts)?;
    };

    Ok(Response::new()
        .may_add_message(if refunds.is_non_empty() {
            Some(Message::transfer(ctx.sender, refunds.clone())?)
        } else {
            None
        })
        .add_event(Repaid {
            user: ctx.sender,
            repaid: ctx.funds,
            refunds,
            remaining_scaled_debts: debts,
        })?)
}

fn claim_pending_protocol_fees(ctx: MutableCtx) -> anyhow::Result<Response> {
    let owner = ctx.querier.query_owner()?;

    let (new_assets, markets) = MARKETS
        .range(ctx.storage, None, None, Order::Ascending)
        .map(|res| -> anyhow::Result<_> {
            let (denom, market) = res?;
            let market = core::update_indices(market, ctx.block.timestamp)?;
            Ok((
                (denom.clone(), market.pending_protocol_fee_scaled),
                (denom, market.reset_pending_protocol_fee()),
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>), _>>()?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Assign the pending protocol fees to the chain owner as assets.
    ASSETS.may_update(ctx.storage, owner, |maybe_assets| -> StdResult<_> {
        let mut assets = maybe_assets.unwrap_or_default();
        for (denom, new_asset) in new_assets {
            let asset = assets.entry(denom).or_default();
            asset.checked_add_assign(new_asset)?;
        }

        Ok(assets)
    })?;

    Ok(Response::new())
}
