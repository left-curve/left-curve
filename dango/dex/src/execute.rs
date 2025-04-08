use {
    crate::{
        FillingOutcome, INCOMING_ORDERS, MatchingOutcome, NEXT_ORDER_ID, ORDERS, Order, PAIRS,
        PassiveLiquidityPool, RESERVES, core, fill_orders, match_orders,
    },
    anyhow::{anyhow, bail, ensure},
    dango_types::{
        bank,
        dex::{
            CreateLimitOrderRequest, Direction, ExecuteMsg, InstantiateMsg, LP_NAMESPACE,
            NAMESPACE, OrderCanceled, OrderFilled, OrderIds, OrderSubmitted, OrdersMatched, PairId,
            PairUpdate, PairUpdated,
        },
    },
    grug::{
        Addr, Coin, CoinPair, Coins, Denom, EventBuilder, GENESIS_SENDER, Inner, Message,
        MultiplyFraction, MutableCtx, Number, Order as IterationOrder, QuerierExt, Response,
        StdResult, Storage, SudoCtx, Udec128, Uint128, UniqueVec,
    },
    std::collections::{BTreeMap, BTreeSet},
};

const HALF: Udec128 = Udec128::new_percent(50);

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> anyhow::Result<Response> {
    batch_update_pairs(ctx, msg.pairs)
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::BatchUpdatePairs(updates) => batch_update_pairs(ctx, updates),
        ExecuteMsg::BatchUpdateOrders { creates, cancels } => {
            batch_update_orders(ctx, creates, cancels)
        },
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

#[inline]
fn batch_update_pairs(ctx: MutableCtx, updates: Vec<PairUpdate>) -> anyhow::Result<Response> {
    ensure!(
        ctx.sender == ctx.querier.query_owner()? || ctx.sender == GENESIS_SENDER,
        "only the owner can update a trading pair parameters"
    );

    let mut events = EventBuilder::with_capacity(updates.len());

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

        events.push(PairUpdated {
            base_denom: update.base_denom,
            quote_denom: update.quote_denom,
        })?;
    }

    Ok(Response::new().add_events(events)?)
}

#[inline]
fn batch_update_orders(
    mut ctx: MutableCtx,
    creates: Vec<CreateLimitOrderRequest>,
    cancels: Option<OrderIds>,
) -> anyhow::Result<Response> {
    let mut deposits = Coins::new();
    let mut refunds = Coins::new();
    let mut events = EventBuilder::new();

    // --------------------------- 1. Cancel orders ----------------------------

    // First, collect all orders to be cancelled into memory.
    let orders = match cancels {
        // Cancel all orders.
        Some(OrderIds::All) => ORDERS
            .idx
            .user
            .prefix(ctx.sender)
            .range(ctx.storage, None, None, IterationOrder::Ascending)
            .map(|order| Ok((order?, false)))
            .chain(
                INCOMING_ORDERS
                    .prefix(ctx.sender)
                    .values(ctx.storage, None, None, IterationOrder::Ascending)
                    .map(|order| Ok((order?, true))),
            )
            .collect::<StdResult<Vec<_>>>()?,
        // Cancel selected orders.
        Some(OrderIds::Some(order_ids)) => order_ids
            .into_iter()
            .map(|order_id| {
                // First see if the order is the persistent storage. If not,
                // check the transient storage.
                if let Some(order) = ORDERS.idx.order_id.may_load(ctx.storage, order_id)? {
                    Ok((order, false))
                } else if let Some(order) =
                    INCOMING_ORDERS.may_load(ctx.storage, (ctx.sender, order_id))?
                {
                    Ok((order, true))
                } else {
                    bail!("order with id `{order_id}` not found");
                }
            })
            .collect::<anyhow::Result<Vec<_>>>()?,
        // Do nothing.
        None => Vec::new(),
    };

    // Now, cancel the orders one by one.
    for ((order_key, order), is_incoming) in orders {
        let ((base_denom, quote_denom), direction, price, order_id) = &order_key;

        ensure!(
            ctx.sender == order.user,
            "only the user can cancel the order"
        );

        let refund = match direction {
            Direction::Bid => Coin {
                denom: quote_denom.clone(),
                amount: order.remaining.checked_mul_dec_floor(*price)?,
            },
            Direction::Ask => Coin {
                denom: base_denom.clone(),
                amount: order.remaining,
            },
        };

        refunds.insert(refund.clone())?;

        events.push(OrderCanceled {
            order_id: *order_id,
            remaining: order.remaining,
            refund,
        })?;

        if is_incoming {
            INCOMING_ORDERS.remove(ctx.storage, (ctx.sender, *order_id));
        } else {
            ORDERS.remove(ctx.storage, order_key)?;
        }
    }

    // --------------------------- 2. Create orders ----------------------------

    for order in creates {
        ensure!(
            PAIRS.has(ctx.storage, (&order.base_denom, &order.quote_denom)),
            "pair not found with base `{}` and quote `{}`",
            order.base_denom,
            order.quote_denom
        );

        let deposit = match order.direction {
            Direction::Bid => Coin {
                denom: order.quote_denom.clone(),
                amount: order.amount.checked_mul_dec_ceil(order.price)?,
            },
            Direction::Ask => Coin {
                denom: order.base_denom.clone(),
                amount: order.amount,
            },
        };

        let (mut order_id, _) = NEXT_ORDER_ID.increment(ctx.storage)?;

        // For BUY orders, invert the order ID. This is necessary for enforcing
        // price-time priority. See the docs on `OrderId` for details.
        if order.direction == Direction::Bid {
            order_id = !order_id;
        }

        deposits.insert(deposit.clone())?;

        events.push(OrderSubmitted {
            order_id,
            user: ctx.sender,
            base_denom: order.base_denom.clone(),
            quote_denom: order.quote_denom.clone(),
            direction: order.direction,
            price: order.price,
            amount: order.amount,
            deposit,
        })?;

        INCOMING_ORDERS.save(
            ctx.storage,
            (ctx.sender, order_id),
            &(
                (
                    (order.base_denom, order.quote_denom),
                    order.direction,
                    order.price,
                    order_id,
                ),
                Order {
                    user: ctx.sender,
                    amount: order.amount,
                    remaining: order.amount,
                },
            ),
        )?;
    }

    // ----------------------------- 3. Wrap it up -----------------------------

    // Compute the amount of tokens that should be sent back to the users.
    //
    // This equals the amount that user has sent to the contract, plus the
    // amount that are to be refunded from the cancaled orders, and the amount
    // that the user is supposed to deposit for creating the new orders.
    ctx.funds
        .insert_many(refunds)?
        .deduct_many(deposits)
        .map_err(|e| anyhow!("insufficient funds for batch updating orders: {e}"))?;

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, ctx.funds)?)
        .add_events(events)?)
}

#[inline]
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

    // Compute the amount of LP tokens to mint.
    let (reserve, lp_mint_amount) = pair.add_liquidity(reserve, lp_token_supply, deposit)?;

    // Save the updated pool reserve.
    RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &reserve)?;

    Ok(Response::new().add_message({
        let bank = ctx.querier.query_bank()?;
        Message::execute(
            bank,
            &bank::ExecuteMsg::Mint {
                to: ctx.sender,
                denom: pair.lp_denom,
                amount: lp_mint_amount,
            },
            Coins::new(), // No funds needed for minting
        )?
    }))
    // TODO: add event
}

/// Withdraw liquidity from a pool. The LP tokens must be sent with the message.
/// The underlying assets will be returned to the sender.
#[inline]
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
                    denom: pair.lp_denom,
                    amount: lp_burn_amount,
                },
                Coins::new(), // No funds needed for burning
            )?
        })
        .add_message(Message::transfer(ctx.sender, refunds)?))
    // TODO: add events
}

#[inline]
fn swap_exact_amount_in(
    ctx: MutableCtx,
    route: UniqueVec<PairId>,
    minimum_output: Option<Uint128>,
) -> anyhow::Result<Response> {
    let (reserves, output) =
        core::swap_exact_amount_in(ctx.storage, route, ctx.funds.into_one_coin()?)?;

    // Ensure the output is above the minimum.
    if let Some(minimum_output) = minimum_output {
        ensure!(
            output.amount >= minimum_output,
            "output amount is below the minimum: {} < {}",
            output.amount,
            minimum_output
        );
    }

    // Save the updated pool reserves.
    for (pair, reserve) in reserves {
        RESERVES.save(ctx.storage, (&pair.base_denom, &pair.quote_denom), &reserve)?;
    }

    // TODO:
    // 1. add events
    // 2. handle the case if output is zero
    Ok(Response::new().add_message(Message::transfer(ctx.sender, output)?))
}

#[inline]
fn swap_exact_amount_out(
    mut ctx: MutableCtx,
    route: UniqueVec<PairId>,
    output: Coin,
) -> anyhow::Result<Response> {
    let (reserves, input) = core::swap_exact_amount_out(ctx.storage, route, output.clone())?;

    // The user must have sent no less than the required input amount.
    // Any extra is refunded.
    ctx.funds.deduct(input)?;
    ctx.funds.insert(output)?;

    // Save the updated pool reserves.
    for (pair, reserve) in reserves {
        RESERVES.save(ctx.storage, (&pair.base_denom, &pair.quote_denom), &reserve)?;
    }

    Ok(Response::new().add_message(Message::transfer(ctx.sender, ctx.funds)?))
}

/// Match and fill orders using the uniform price auction strategy.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    let mut events = EventBuilder::new();
    let mut refunds = BTreeMap::new();

    // Collect incoming orders and clear the temporary storage.
    let incoming_orders = INCOMING_ORDERS.drain(ctx.storage, None, None)?;

    // Add incoming orders to the persistent storage.
    for (order_key, order) in incoming_orders.values() {
        ORDERS.save(ctx.storage, order_key.clone(), order)?;
    }

    // Find all the unique pairs that have received new orders in the block.
    let pairs = incoming_orders
        .into_values()
        .map(|((pair, ..), _)| pair)
        .collect::<BTreeSet<_>>();

    // Loop through the pairs that have received new orders in the block.
    // Match and clear the orders for each of them.
    // TODO: spawn a thread for each pair to process them in parallel.
    for (base_denom, quote_denom) in pairs {
        clear_orders_of_pair(
            ctx.storage,
            base_denom,
            quote_denom,
            &mut events,
            &mut refunds,
        )?;
    }

    Response::new()
        .may_add_message(if !refunds.is_empty() {
            Some(Message::batch_transfer(refunds)?)
        } else {
            None
        })
        .add_events(events)
}

#[inline]
fn clear_orders_of_pair(
    storage: &mut dyn Storage,
    base_denom: Denom,
    quote_denom: Denom,
    events: &mut EventBuilder,
    refunds: &mut BTreeMap<Addr, Coins>,
) -> StdResult<()> {
    // Iterate BUY orders from the highest price to the lowest.
    // Iterate SELL orders from the lowest price to the highest.
    let bid_iter = ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Bid)
        .range(storage, None, None, IterationOrder::Descending);
    let ask_iter = ORDERS
        .prefix((base_denom.clone(), quote_denom.clone()))
        .append(Direction::Ask)
        .range(storage, None, None, IterationOrder::Ascending);

    // Run the order matching algorithm.
    let MatchingOutcome {
        range,
        volume,
        bids,
        asks,
    } = match_orders(bid_iter, ask_iter)?;

    // If no matching orders were found, then we're done with this pair.
    // Continue to the next pair.
    let Some((lower_price, higher_price)) = range else {
        return Ok(());
    };

    // Choose the clearing price. Any price within `range` gives the same
    // volume (measured in the base asset). We can either take
    //
    // - the lower end,
    // - the higher end, or
    // - the midpoint of the range.
    //
    // Here we choose the midpoint.
    let clearing_price = lower_price.checked_add(higher_price)?.checked_mul(HALF)?;

    events.push(OrdersMatched {
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        clearing_price,
        volume,
    })?;

    // Clear the BUY orders.
    for FillingOutcome {
        order_direction,
        order_price,
        order_id,
        order,
        filled,
        cleared,
        refund_base,
        refund_quote,
    } in fill_orders(bids, asks, clearing_price, volume)?
    {
        let refund = Coins::try_from([
            Coin {
                denom: base_denom.clone(),
                amount: refund_base,
            },
            Coin {
                denom: quote_denom.clone(),
                amount: refund_quote,
            },
        ])?;

        events.push(OrderFilled {
            order_id,
            clearing_price,
            filled,
            refund: refund.clone(),
            fee: None,
            cleared,
        })?;

        refunds.entry(order.user).or_default().insert_many(refund)?;

        if cleared {
            ORDERS.remove(
                storage,
                (
                    (base_denom.clone(), quote_denom.clone()),
                    order_direction,
                    order_price,
                    order_id,
                ),
            )?;
        } else {
            ORDERS.save(
                storage,
                (
                    (base_denom.clone(), quote_denom.clone()),
                    order_direction,
                    order_price,
                    order_id,
                ),
                &order,
            )?;
        }
    }

    Ok(())
}
