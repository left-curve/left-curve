use {
    crate::{DEBTS, MARKETS, core},
    anyhow::{anyhow, ensure},
    dango_account_factory::ACCOUNTS,
    dango_types::{
        DangoQuerier, bank,
        lending::{
            Borrowed, ExecuteMsg, InstantiateMsg, Market, MarketUpdates, NAMESPACE, Repaid,
            SUBNAMESPACE,
        },
    },
    grug::{
        Coins, Denom, Inner, Message, MutableCtx, NonEmpty, Order, QuerierExt, Response, StdResult,
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
            &Market::new(
                denom.prepend(&[&NAMESPACE, &SUBNAMESPACE])?,
                interest_rate_model,
            ),
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
    updates: BTreeMap<Denom, MarketUpdates>,
) -> anyhow::Result<Response> {
    // Ensure only chain owner can update markets denoms.
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "Only the owner can whitelist denoms"
    );

    for (denom, updates) in updates {
        if let Some(market) = MARKETS.may_load(ctx.storage, &denom)? {
            if let Some(interest_rate_model) = updates.interest_rate_model {
                // Update indexes first, so that interests accumulated up to this
                // point are accounted for. Then, set the new interest rate model.
                let new_market = market
                    .update_indices(&ctx.querier, ctx.block.timestamp)?
                    .set_interest_rate_model(interest_rate_model);

                MARKETS.save(ctx.storage, &denom, &new_market)?;
            }
        } else {
            MARKETS.save(
                ctx.storage,
                &denom,
                &Market::new(
                    denom.prepend(&[&NAMESPACE, &SUBNAMESPACE])?,
                    updates.interest_rate_model.ok_or_else(|| {
                        anyhow!(
                            "interest rate model is required when adding new market {}",
                            denom
                        )
                    })?,
                ),
            )?;
        }
    }

    Ok(Response::new())
}

fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Immutably update markets and compute the amount of LP tokens to mint.
    let (lp_tokens, markets) = core::deposit(
        ctx.storage,
        &ctx.querier,
        ctx.block.timestamp,
        ctx.funds.clone(),
    )?;

    // Save the updated markets.
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Mint the LP tokens to the sender.
    let bank = ctx.querier.query_bank()?;
    let msgs = lp_tokens
        .into_iter()
        .map(|coin| {
            Message::execute(
                bank,
                &bank::ExecuteMsg::Mint {
                    to: ctx.sender,
                    denom: coin.denom,
                    amount: coin.amount,
                },
                Coins::new(),
            )
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::new().add_messages(msgs))
}

fn withdraw(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Immutably update markets and compute the amount of underlying coins to withdraw
    let (withdrawn, markets) = core::withdraw(
        ctx.storage,
        &ctx.querier,
        ctx.block.timestamp,
        ctx.funds.clone(),
    )?;

    // Save the updated markets
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Burn the LP tokens.
    let bank = ctx.querier.query_bank()?;
    let msgs = ctx
        .funds
        .into_iter()
        .map(|coin| {
            Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    denom: coin.denom,
                    amount: coin.amount,
                },
                Coins::new(),
            )
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_message(Message::transfer(ctx.sender, withdrawn)?))
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
        &ctx.querier,
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
        .add_message(Message::transfer(ctx.sender, coins.inner().clone())?)
        .add_event(Borrowed {
            user: ctx.sender,
            borrowed: coins.into_inner(),
        })?)
}

fn repay(ctx: MutableCtx) -> anyhow::Result<Response> {
    let (scaled_debts, markets, refunds) = core::repay(
        ctx.storage,
        &ctx.querier,
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
        .may_add_message(if refunds.is_non_empty() {
            Some(Message::transfer(ctx.sender, refunds.clone())?)
        } else {
            None
        })
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

    let (msgs, markets) = MARKETS
        .range(ctx.storage, None, None, Order::Ascending)
        .map(|res| -> anyhow::Result<_> {
            let (denom, market) = res?;
            let market = market.update_indices(&ctx.querier, ctx.block.timestamp)?;
            Ok((
                Message::execute(
                    bank,
                    &bank::ExecuteMsg::Mint {
                        to: owner,
                        denom: market.supply_lp_denom.clone(),
                        amount: market.pending_protocol_fee_scaled,
                    },
                    Coins::new(),
                )?,
                (denom, market),
            ))
        })
        .collect::<Result<(Vec<_>, Vec<_>), _>>()?;

    // Reset the pending protocol fees and save the updated markets
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market.reset_pending_protocol_fee())?;
    }

    Ok(Response::new().add_messages(msgs))
}
