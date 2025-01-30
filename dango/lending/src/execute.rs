use {
    crate::{query_preview_deposit, query_preview_withdraw, DEBTS, MARKETS},
    anyhow::{anyhow, ensure, Ok},
    dango_account_factory::ACCOUNTS,
    dango_types::{
        bank,
        lending::{ExecuteMsg, InstantiateMsg, Market, MarketUpdates, NAMESPACE, SUBNAMESPACE},
        DangoQuerier,
    },
    grug::{
        Coin, Coins, Denom, Inner, Message, MutableCtx, Number, NumberConst, QuerierExt, Response,
        StdResult, StorageQuerier, Udec128, Uint128,
    },
    std::collections::BTreeMap,
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    for (denom, updates) in msg.markets {
        let interest_rate_model = updates
            .interest_rate_model
            .ok_or_else(|| anyhow!("interest rate model is required for market {}", denom))?;

        MARKETS.save(ctx.storage, &denom, &Market {
            supply_lp_denom: denom.prepend(&[&NAMESPACE, &SUBNAMESPACE])?,
            interest_rate_model,
            total_borrowed_scaled: Udec128::ZERO,
            total_supplied_scaled: Uint128::ZERO,
            borrow_index: Udec128::ONE,
            supply_index: Udec128::ONE,
            last_update_time: ctx.block.timestamp,
        })?;
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

    for (denom, updates) in updates {
        let market = MARKETS.may_load(ctx.storage, &denom)?;

        if let Some(market) = market {
            if let Some(interest_rate_model) = updates.interest_rate_model {
                MARKETS.save(ctx.storage, &denom, &Market {
                    interest_rate_model,
                    ..market
                })?;
            }
        } else {
            MARKETS.save(ctx.storage, &denom, &Market {
                supply_lp_denom: denom.prepend(&[&NAMESPACE, &SUBNAMESPACE])?,
                interest_rate_model: updates.interest_rate_model.ok_or_else(|| {
                    anyhow!(
                        "interest rate model is required when adding new market {}",
                        denom
                    )
                })?,
                total_borrowed_scaled: Udec128::ZERO,
                total_supplied_scaled: Uint128::ZERO,
                borrow_index: Udec128::ONE,
                supply_index: Udec128::ONE,
                last_update_time: ctx.block.timestamp,
            })?;
        }
    }

    Ok(Response::new())
}

fn deposit(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Immutably update markets and compute the amount of LP tokens to mint
    let (sender_lp_tokens, protocol_lp_tokens, markets) =
        query_preview_deposit(ctx.storage, ctx.block.timestamp, ctx.funds.clone())?;

    // Save the updated markets
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Mint the LP tokens to the sender
    let bank = ctx.querier.query_bank()?;
    let mut msgs = sender_lp_tokens
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

    // Mint the protocol LP tokens to the owner
    let owner = ctx.querier.query_owner()?;
    msgs.extend(
        protocol_lp_tokens
            .into_iter()
            .map(|coin| {
                Message::execute(
                    bank,
                    &bank::ExecuteMsg::Mint {
                        to: owner,
                        denom: coin.denom,
                        amount: coin.amount,
                    },
                    Coins::new(),
                )
            })
            .collect::<StdResult<Vec<_>>>()?,
    );

    Ok(Response::new().add_messages(msgs))
}

fn withdraw(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Immutably update markets and compute the amount of underlying coins to withdraw
    let (withdrawn, protocol_lp_tokens, markets) =
        query_preview_withdraw(ctx.storage, ctx.block.timestamp, ctx.funds.clone())?;

    // Save the updated markets
    for (denom, market) in markets {
        MARKETS.save(ctx.storage, &denom, &market)?;
    }

    // Burn the LP tokens
    let bank = ctx.querier.query_bank()?;
    let mut msgs = ctx
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

    // Mint the protocol LP tokens to the owner
    let owner = ctx.querier.query_owner()?;
    msgs.extend(
        protocol_lp_tokens
            .into_iter()
            .map(|coin| {
                Message::execute(
                    bank,
                    &bank::ExecuteMsg::Mint {
                        to: owner,
                        denom: coin.denom,
                        amount: coin.amount,
                    },
                    Coins::new(),
                )
            })
            .collect::<StdResult<Vec<_>>>()?,
    );

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
            .query_wasm_path(account_factory, ACCOUNTS.path(ctx.sender))?
            .params
            .is_margin(),
        "Only margin accounts can borrow and repay"
    );

    // Load the sender's debts
    let mut scaled_debts = DEBTS.may_load(ctx.storage, ctx.sender)?.unwrap_or_default();

    let bank = ctx.querier.query_bank()?;
    let owner = ctx.querier.query_owner()?;
    let mut protocol_lp_mint_msgs = vec![];

    for coin in coins.clone() {
        // Update the market state
        let (market, protocol_fee_scaled) = MARKETS
            .load(ctx.storage, &coin.denom)?
            .update_indices(ctx.block.timestamp)?;

        // Update the sender's liabilities
        let prev_scaled_debt = scaled_debts.get(&coin.denom).cloned().unwrap_or_default();
        let new_scaled_debt =
            Udec128::new(coin.amount.into_inner()).checked_div(market.borrow_index)?;
        let added_scaled_debt = prev_scaled_debt.checked_add(new_scaled_debt)?;
        scaled_debts.insert(coin.denom.clone(), added_scaled_debt);

        // Save the updated market state
        let market = market.add_borrowed(added_scaled_debt)?;
        MARKETS.save(ctx.storage, &coin.denom, &market)?;

        // Mint the protocol LP tokens to the owner
        protocol_lp_mint_msgs.push(Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: owner,
                denom: market.supply_lp_denom,
                amount: protocol_fee_scaled,
            },
            Coins::new(),
        )?);
    }

    // Save the updated debts
    DEBTS.save(ctx.storage, ctx.sender, &scaled_debts)?;

    // Transfer the coins to the caller
    Ok(Response::new()
        .add_messages(protocol_lp_mint_msgs)
        .add_message(Message::transfer(ctx.sender, coins)?))
}

fn repay(ctx: MutableCtx) -> anyhow::Result<Response> {
    let mut refunds = Coins::new();

    // Read debts
    let mut scaled_debts = DEBTS.may_load(ctx.storage, ctx.sender)?.unwrap_or_default();

    let bank = ctx.querier.query_bank()?;
    let owner = ctx.querier.query_owner()?;
    let mut protocol_lp_mint_msgs = vec![];

    for coin in ctx.funds {
        // Update the market indices
        let (market, protocol_fee_scaled) = MARKETS
            .load(ctx.storage, &coin.denom)?
            .update_indices(ctx.block.timestamp)?;

        // Calculated the users real debt
        let scaled_debt = scaled_debts.get(&coin.denom).cloned().unwrap_or_default();
        let debt = market.calculate_debt(scaled_debt)?;
        // Refund the remainders to the sender, if any.
        let repaid = if coin.amount > debt {
            let refund_amount = coin.amount.checked_sub(debt)?;
            refunds.insert(Coin::new(coin.denom.clone(), refund_amount)?)?;
            debt
        } else {
            coin.amount
        };

        let debt_after = debt.checked_sub(repaid)?;

        let debt_after_scaled =
            Udec128::new(debt_after.into_inner()).checked_div(market.borrow_index)?;

        let scaled_debt_diff = scaled_debt.checked_sub(debt_after_scaled)?;

        // Update the sender's liabilities
        let repaid_debt_scaled =
            Udec128::new(repaid.into_inner()).checked_div(market.borrow_index)?;
        scaled_debts.insert(
            coin.denom.clone(),
            scaled_debt.saturating_sub(repaid_debt_scaled),
        );

        // Deduct the repaid debt and save the updated market state
        MARKETS.save(
            ctx.storage,
            &coin.denom,
            &market.deduct_borrowed(scaled_debt_diff)?,
        )?;

        // Mint the protocol LP tokens to the owner
        protocol_lp_mint_msgs.push(Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: owner,
                denom: market.supply_lp_denom,
                amount: protocol_fee_scaled,
            },
            Coins::new(),
        )?);
    }

    // Save the updated debts
    DEBTS.save(ctx.storage, ctx.sender, &scaled_debts)?;

    Ok(Response::new()
        .add_messages(protocol_lp_mint_msgs)
        .add_message(Message::transfer(ctx.sender, refunds)?))
}
