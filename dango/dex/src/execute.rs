use {
    crate::{
        fill_orders, match_orders, FillingOutcome, MatchingOutcome, Order, PassiveLiquidityPool,
        INCOMING_ORDERS, NEXT_ORDER_ID, ORDERS, PAIRS, RESERVES,
    },
    anyhow::{anyhow, bail, ensure},
    dango_types::{
        bank,
        dex::{
            CreateLimitOrderRequest, Direction, ExecuteMsg, InstantiateMsg, OrderCanceled,
            OrderFilled, OrderIds, OrderSubmitted, OrdersMatched, PairUpdate, PairUpdated,
            SlippageControl, SwapRoute, LP_NAMESPACE, NAMESPACE,
        },
    },
    grug::{
        Addr, Coin, CoinPair, Coins, Denom, EventBuilder, Message, MultiplyFraction, MutableCtx,
        Number, Order as IterationOrder, QuerierExt, Response, StdResult, Storage, SudoCtx,
        Udec128, Uint128, GENESIS_SENDER,
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
        ExecuteMsg::Swap {
            amount,
            direction,
            route,
            slippage,
        } => swap(ctx, amount, direction, SwapRoute::new(route), slippage),
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

    Ok(Response::new().add_events(events))
}

/// Internal use function for handling the logic of batch updating orders.
///
/// ## Inputs
///
/// - `ctx`: The context of the contract.
/// - `creates`: The orders to create.
/// - `cancels`: The orders to cancel.
///
/// ## Outputs
///
/// - A `Coins` instance containing the deposits required to create the orders.
/// - A `Coins` instance containing the refunds for the cancelled orders.
/// - A vec of `ContractEvent` instances containing the events.
fn _batch_update_orders(
    storage: &mut dyn Storage,
    sender: Addr,
    creates: Vec<CreateLimitOrderRequest>,
    cancels: Option<OrderIds>,
) -> anyhow::Result<(Coins, Coins, EventBuilder)> {
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
            .prefix(sender)
            .range(storage, None, None, IterationOrder::Ascending)
            .map(|order| Ok((order?, false)))
            .chain(
                INCOMING_ORDERS
                    .prefix(sender)
                    .values(storage, None, None, IterationOrder::Ascending)
                    .map(|order| Ok((order?, true))),
            )
            .collect::<StdResult<Vec<_>>>()?,
        // Cancel selected orders.
        Some(OrderIds::Some(order_ids)) => order_ids
            .into_iter()
            .map(|order_id| {
                // First see if the order is the persistent storage. If not,
                // check the transient storage.
                if let Some(order) = ORDERS.idx.order_id.may_load(storage, order_id)? {
                    Ok((order, false))
                } else if let Some(order) = INCOMING_ORDERS.may_load(storage, (sender, order_id))? {
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

        ensure!(sender == order.user, "only the user can cancel the order");

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
            INCOMING_ORDERS.remove(storage, (sender, *order_id));
        } else {
            ORDERS.remove(storage, order_key)?;
        }
    }

    // --------------------------- 2. Create orders ----------------------------

    for order in creates {
        ensure!(
            PAIRS.has(storage, (&order.base_denom, &order.quote_denom)),
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

        let (mut order_id, _) = NEXT_ORDER_ID.increment(storage)?;

        // For BUY orders, invert the order ID. This is necessary for enforcing
        // price-time priority. See the docs on `OrderId` for details.
        if order.direction == Direction::Bid {
            order_id = !order_id;
        }

        deposits.insert(deposit.clone())?;

        events.push(OrderSubmitted {
            order_id,
            user: sender,
            base_denom: order.base_denom.clone(),
            quote_denom: order.quote_denom.clone(),
            direction: order.direction,
            price: order.price,
            amount: order.amount,
            deposit,
        })?;

        INCOMING_ORDERS.save(
            storage,
            (sender, order_id),
            &(
                (
                    (order.base_denom, order.quote_denom),
                    order.direction,
                    order.price,
                    order_id,
                ),
                Order {
                    user: sender,
                    amount: order.amount,
                    remaining: order.amount,
                },
            ),
        )?;
    }

    Ok((deposits, refunds, events))
}

#[inline]
fn batch_update_orders(
    ctx: MutableCtx,
    creates: Vec<CreateLimitOrderRequest>,
    cancels: Option<OrderIds>,
) -> anyhow::Result<Response> {
    let mut funds = ctx.funds.clone();
    let sender = ctx.sender;

    let (deposits, refunds, events) = _batch_update_orders(ctx.storage, sender, creates, cancels)?;

    // Compute the amount of tokens that should be sent back to the users.
    //
    // This equals the amount that user has sent to the contract, plus the
    // amount that are to be refunded from the cancelled orders, and the amount
    // that the user is supposed to deposit for creating the new orders.
    funds
        .insert_many(refunds)?
        .deduct_many(deposits)
        .map_err(|e| anyhow!("insufficient funds for batch updating orders: {e}"))?;

    Ok(Response::new()
        .add_message(Message::transfer(sender, funds)?)
        .add_events(events))
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

/// Perform a swap in the pool.
#[inline]
fn swap(
    ctx: MutableCtx,
    amount: Uint128,
    direction: Direction,
    route: SwapRoute,
    slippage: Option<SlippageControl>,
) -> anyhow::Result<Response> {
    let mut funds = ctx.funds.clone();

    // Validate the route.
    route.validate()?;

    let (mut coin_in, mut coin_out) = match direction {
        Direction::Bid => {
            let coin_in = Coin::new(route.end().clone(), amount)?;
            let coin_out = Coin::new(route.start().clone(), amount)?;
            (coin_in, coin_out)
        },
        Direction::Ask => {
            let coin_in = Coin::new(route.start().clone(), amount)?;
            let coin_out = Coin::new(route.end().clone(), amount)?;
            (coin_in, coin_out)
        },
    };

    for (base_denom, quote_denom) in route.clone().into_iter().zip(route.into_iter().skip(1)) {
        // Load the pair params.
        let (pair, reserves, reverse_base_quote) =
            match PAIRS.may_load(ctx.storage, (&base_denom, &quote_denom))? {
                Some(pair) => (
                    pair,
                    RESERVES.load(ctx.storage, (&base_denom, &quote_denom))?,
                    false,
                ),
                None => (
                    PAIRS.load(ctx.storage, (&quote_denom, &base_denom))?,
                    RESERVES.load(ctx.storage, (&quote_denom, &base_denom))?,
                    true,
                ),
            };

        // For direction Ask use the output of last swap as the input for the next swap.
        // For direction Bid use the input of last swap as the demanded output for the
        // next swap.
        let (new_reserves, offer, ask) = match direction {
            Direction::Bid => pair.swap(
                reserves,
                base_denom.clone(),
                quote_denom.clone(),
                direction,
                coin_in.amount,
            )?,
            Direction::Ask => pair.swap(
                reserves,
                base_denom.clone(),
                quote_denom.clone(),
                direction,
                coin_out.amount,
            )?,
        };

        // Update the coin in and out.
        match direction {
            Direction::Bid => {
                coin_in = offer.clone();
            },
            Direction::Ask => {
                coin_out = ask.clone();
            },
        }

        // Save the updated pool reserves.
        if reverse_base_quote {
            RESERVES.save(ctx.storage, (&quote_denom, &base_denom), &new_reserves)?;
        } else {
            RESERVES.save(ctx.storage, (&base_denom, &quote_denom), &new_reserves)?;
        }
    }

    // Enforce slippage control.
    if let Some(slippage_control) = slippage {
        match slippage_control {
            SlippageControl::MinimumOut(min_out) => {
                ensure!(
                    direction != Direction::Bid,
                    "minimum out is only supported for direction: ask"
                );
                ensure!(coin_out.amount >= min_out, "slippage tolerance exceeded");
            },
            SlippageControl::MaximumIn(max_in) => {
                ensure!(
                    direction != Direction::Ask,
                    "maximum in is only supported for direction: bid"
                );
                ensure!(coin_in.amount <= max_in, "slippage tolerance exceeded");
            },
            SlippageControl::PriceLimit(price_limit) => {
                let execution_price = Udec128::checked_from_ratio(coin_out.amount, coin_in.amount)?;
                match direction {
                    Direction::Bid => ensure!(
                        execution_price <= price_limit,
                        "slippage tolerance exceeded"
                    ),
                    Direction::Ask => ensure!(
                        execution_price >= price_limit,
                        "slippage tolerance exceeded"
                    ),
                }
            },
        }
    }

    // Deduct the coin in and add the coin out to the funds.
    funds
        .deduct(coin_in)
        .map_err(|_| anyhow::anyhow!("insufficient funds"))?;
    funds.insert(coin_out)?;

    // Send back any unused funds together with proceeds from swaps
    Ok(Response::new().add_message(Message::transfer(ctx.sender, funds)?))
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

/// Match and fill orders using the uniform price auction strategy.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> anyhow::Result<Response> {
    let mut refunds = BTreeMap::new();
    let mut creates = Vec::new();

    // Loop through all passive pools and reflect the pools onto the orderbook
    let pairs_with_pools = RESERVES
        .range(ctx.storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    for ((base_denom, quote_denom), reserve) in pairs_with_pools {
        let pair = PAIRS.load(ctx.storage, (&base_denom, &quote_denom))?;
        let creates_for_pair = pair.reflect_curve(base_denom, quote_denom, &reserve)?;

        // Cancel all old orders and submit new ones based on the curve.
        creates.extend(creates_for_pair);
    }

    for (i, create) in creates.iter().enumerate() {
        println!("create {} : {:?}", i, create);
    }

    // Update the orders.
    let (.., mut events) =
        _batch_update_orders(ctx.storage, ctx.contract, creates, Some(OrderIds::All))?;

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

    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::BatchTransfer(refunds),
                Coins::new(),
            )?
        })
        .add_events(events))
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
