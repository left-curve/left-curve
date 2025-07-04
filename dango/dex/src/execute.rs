mod order_cancellation;
mod order_creation;

use {
    crate::{MAX_ORACLE_STALENESS, PAIRS, PassiveLiquidityPool, RESERVES, core},
    anyhow::{anyhow, ensure},
    dango_oracle::OracleQuerier,
    dango_types::{
        DangoQuerier, bank,
        dex::{
            CancelOrderRequest, CreateLimitOrderRequest, CreateMarketOrderRequest, ExecuteMsg,
            InstantiateMsg, LP_NAMESPACE, NAMESPACE, PairId, PairUpdate, Swapped,
        },
        taxman::{self, FeeType},
    },
    grug::{
        Coin, CoinPair, Coins, Denom, EventBuilder, GENESIS_SENDER, Inner, IsZero, Message,
        MutableCtx, NonZero, QuerierExt, Response, Uint128, UniqueVec, btree_map, coins,
    },
};

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    batch_update_pairs(ctx, msg.pairs)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::BatchUpdatePairs(updates) => batch_update_pairs(ctx, updates),
        ExecuteMsg::BatchUpdateOrders {
            creates_market,
            creates_limit,
            cancels,
        } => batch_update_orders(ctx, creates_market, creates_limit, cancels),
        ExecuteMsg::ProvideLiquidity {
            base_denom,
            quote_denom,
        } => provide_liquidity(ctx, base_denom, quote_denom),
        ExecuteMsg::WithdrawLiquidity {
            base_denom,
            quote_denom,
        } => withdraw_liquidity(ctx, base_denom, quote_denom),
        ExecuteMsg::SwapExactAmountIn {
            route,
            minimum_output,
        } => swap_exact_amount_in(ctx, route.into_inner(), minimum_output),
        ExecuteMsg::SwapExactAmountOut { route, output } => {
            swap_exact_amount_out(ctx, route.into_inner(), output)
        },
    }
}

fn batch_update_pairs(ctx: MutableCtx, updates: Vec<PairUpdate>) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()? || ctx.sender == GENESIS_SENDER,
        "only the owner can update a trading pair parameters"
    );

    for update in updates {
        ensure!(
            update
                .params
                .lp_denom
                .starts_with(&[NAMESPACE.clone(), LP_NAMESPACE.clone()]),
            "LP token denom doesn't start with the correct prefix: `{}/{}/...`",
            NAMESPACE.as_ref(),
            LP_NAMESPACE.as_ref()
        );

        PAIRS.save(
            ctx.storage,
            (&update.base_denom, &update.quote_denom),
            &update.params,
        )?;
    }

    Ok(Response::new())
}

fn batch_update_orders(
    mut ctx: MutableCtx,
    creates_market: Vec<CreateMarketOrderRequest>,
    creates_limit: Vec<CreateLimitOrderRequest>,
    cancels: Option<CancelOrderRequest>,
) -> anyhow::Result<Response> {
    let mut deposits = Coins::new();
    let mut refunds = Coins::new();
    let mut events = EventBuilder::new();

    match cancels {
        // Cancel selected orders.
        Some(CancelOrderRequest::Some(order_ids)) => {
            for order_id in order_ids {
                order_cancellation::cancel_order_from_user(
                    ctx.storage,
                    ctx.sender,
                    order_id,
                    &mut events,
                    &mut refunds,
                )?;
            }
        },
        // Cancel all orders.
        Some(CancelOrderRequest::All) => {
            order_cancellation::cancel_all_orders_from_user(
                ctx.storage,
                ctx.sender,
                &mut events,
                &mut refunds,
            )?;
        },
        // Do nothing.
        None => {},
    };

    for order in creates_market {
        order_creation::create_market_order(
            ctx.storage,
            ctx.sender,
            order,
            &mut events,
            &mut deposits,
        )?;
    }

    for order in creates_limit {
        order_creation::create_limit_order(
            ctx.storage,
            ctx.block.height,
            ctx.sender,
            order,
            &mut events,
            &mut deposits,
        )?;
    }

    // Compute the amount of tokens that should be sent back to the users.
    //
    // This equals the amount that user has sent to the contract, plus the
    // amount that are to be refunded from the cancelled orders, and the amount
    // that the user is supposed to deposit for creating the new orders.
    ctx.funds
        .insert_many(refunds)?
        .deduct_many(deposits)
        .map_err(|e| anyhow!("insufficient funds for batch updating orders: {e}"))?;

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, ctx.funds)?)
        .add_events(events)?)
}

fn provide_liquidity(
    mut ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
) -> anyhow::Result<Response> {
    // Get the deposited funds.
    let deposit = ctx
        .funds
        .take_pair((base_denom.clone(), quote_denom.clone()))?;

    // The user must have not sent any funds other the base/quote denoms.
    ensure!(
        ctx.funds.is_empty(),
        "unexpected deposit: {}; expecting `{}` and `{}`",
        ctx.funds,
        base_denom,
        quote_denom
    );

    // Load the pair params.
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Load the current pool reserve. Default to empty if not found.
    let reserve = RESERVES
        .may_load(ctx.storage, (&base_denom, &quote_denom))?
        .map_or_else(
            || CoinPair::new_empty(base_denom.clone(), quote_denom.clone()),
            Ok,
        )?;

    // Query the LP token supply.
    let lp_token_supply = ctx.querier.query_supply(pair.lp_denom.clone())?;

    // Create the oracle querier with max staleness.
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // Compute the amount of LP tokens to mint.
    let (reserve, lp_mint_amount) =
        pair.add_liquidity(&mut oracle_querier, reserve, lp_token_supply, deposit)?;

    // Save the updated pool reserve.
    RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;

    Ok(Response::new().add_message({
        let bank = ctx.querier.query_bank()?;
        Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.sender,
                coins: coins! { pair.lp_denom => lp_mint_amount },
            },
            Coins::new(), // No funds needed for minting
        )?
    }))
    // TODO: add event
}

/// Withdraw liquidity from a pool. The LP tokens must be sent with the message.
/// The underlying assets will be returned to the sender.
fn withdraw_liquidity(
    mut ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
) -> anyhow::Result<Response> {
    // Load the pair params.
    let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Load the current pool reserve.
    let reserve = RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?;

    // Query the LP token supply.
    let lp_token_supply = ctx.querier.query_supply(pair.lp_denom.clone())?;

    // Get the sent LP tokens.
    let lp_burn_amount = ctx.funds.take(pair.lp_denom.clone()).amount;

    // The user must have not sent any funds other the LP token.
    ensure!(
        ctx.funds.is_empty(),
        "unexpected deposit: {}; expecting `{}`",
        ctx.funds,
        pair.lp_denom
    );

    // Calculate the amount of each asset to return
    let (reserve, refunds) = pair.remove_liquidity(reserve, lp_token_supply, lp_burn_amount)?;

    // Save the updated pool reserve.
    RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;

    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::Burn {
                    from: ctx.contract,
                    coins: coins! { pair.lp_denom => lp_burn_amount },
                },
                Coins::new(), // No funds needed for burning
            )?
        })
        .add_message(Message::transfer(ctx.sender, refunds)?))
    // TODO: add events
}

fn swap_exact_amount_in(
    ctx: MutableCtx,
    route: UniqueVec<PairId>,
    minimum_output: Option<Uint128>,
) -> anyhow::Result<Response> {
    let input = ctx.funds.into_one_coin()?;
    let app_cfg = ctx.querier.query_dango_config()?;

    // Create the oracle querier with max staleness.
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // Perform the swap.
    let (reserves, output, protocol_fee) = core::swap_exact_amount_in(
        ctx.storage,
        &mut oracle_querier,
        *app_cfg.taker_fee_rate, // Charge the taker fee rate for swaps.
        route,
        input.clone(),
    )?;

    // Ensure the output is above the minimum.
    // If not minimum is specified, the output should at least be greater than zero.
    if let Some(minimum_output) = minimum_output {
        ensure!(
            output.amount >= minimum_output,
            "output amount is below the minimum: {} < {}",
            output.amount,
            minimum_output
        );
    } else {
        ensure!(output.amount.is_non_zero(), "output amount is zero");
    }

    // Save the updated pool reserves.
    for (pair, reserve) in reserves {
        RESERVES.save(ctx.storage, (&pair.base_denom, &pair.quote_denom), &reserve)?;
    }

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, output.clone())?)
        .may_add_message(if protocol_fee.is_non_zero() {
            Some(Message::execute(
                app_cfg.addresses.taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Trade,
                    payments: btree_map! {
                        ctx.sender => coins! { output.denom.clone() => protocol_fee },
                    },
                },
                coins! { output.denom.clone() => protocol_fee },
            )?)
        } else {
            None
        })
        .add_event(Swapped {
            user: ctx.sender,
            input,
            output,
        })?)
}

fn swap_exact_amount_out(
    mut ctx: MutableCtx,
    route: UniqueVec<PairId>,
    output: NonZero<Coin>,
) -> anyhow::Result<Response> {
    let app_cfg = ctx.querier.query_dango_config()?;

    // Create the oracle querier with max staleness.
    let mut oracle_querier = OracleQuerier::new_remote(ctx.querier.query_oracle()?, ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // Perform the swap.
    let (reserves, input, protocol_fee) = core::swap_exact_amount_out(
        ctx.storage,
        &mut oracle_querier,
        *app_cfg.taker_fee_rate, // Charge the taker fee rate for swaps.
        route,
        output.clone(),
    )?;

    // The user must have sent no less than the required input amount.
    // Any extra is refunded.
    ctx.funds
        .insert(output.clone().into_inner())?
        .deduct(input.clone())
        .map_err(|e| anyhow!("insufficient input for swap: {e}"))?;

    // Save the updated pool reserves.
    for (pair, reserve) in reserves {
        RESERVES.save(ctx.storage, (&pair.base_denom, &pair.quote_denom), &reserve)?;
    }

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, ctx.funds)?)
        .may_add_message(if protocol_fee.is_non_zero() {
            Some(Message::execute(
                app_cfg.addresses.taxman,
                &taxman::ExecuteMsg::Pay {
                    ty: FeeType::Trade,
                    payments: btree_map! {
                        ctx.sender => coins! { output.denom.clone() => protocol_fee },
                    },
                },
                coins! { output.denom.clone() => protocol_fee },
            )?)
        } else {
            None
        })
        .add_event(Swapped {
            user: ctx.sender,
            input,
            output: output.into_inner(),
        })?)
}
