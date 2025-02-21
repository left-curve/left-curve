use {
    crate::{
        fill_orders, match_orders, FillingOutcome, MatchingOutcome, Order, PassiveLiquidityPool,
        INCOMING_ORDERS, LP_DENOMS, NEXT_ORDER_ID, ORDERS, PAIRS, POOLS,
    },
    anyhow::{bail, ensure},
    dango_types::{
        bank,
        dex::{
            CurveInvariant, Direction, ExecuteMsg, InstantiateMsg, OrderCanceled, OrderFilled,
            OrderIds, OrderSubmitted, OrdersMatched, PairUpdate, PairUpdated, Swap, LP_NAMESPACE,
            NAMESPACE,
        },
    },
    grug::{
        Addr, Coin, CoinPair, Coins, ContractEvent, Denom, EventName, Inner, IsZero, Message,
        MultiplyFraction, MutableCtx, Number, NumberConst, Order as IterationOrder, QuerierExt,
        Response, StdResult, Storage, SudoCtx, Udec128, Uint128,
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
        ExecuteMsg::BatchUpdatePairs(updates) => {
            ensure!(
                ctx.sender == ctx.querier.query_owner()?,
                "only the owner can update a trading pair parameters"
            );

            batch_update_pairs(ctx, updates)
        },
        ExecuteMsg::CreatePassivePool {
            base_denom,
            quote_denom,
            curve_type,
            lp_denom,
            swap_fee,
        } => create_passive_pool(ctx, base_denom, quote_denom, curve_type, lp_denom, swap_fee),
        ExecuteMsg::SubmitOrder {
            base_denom,
            quote_denom,
            direction,
            amount,
            price,
        } => submit_order(ctx, base_denom, quote_denom, direction, amount, price),
        ExecuteMsg::BatchSwap { swaps } => batch_swap(ctx, swaps),
        ExecuteMsg::CancelOrders { order_ids } => cancel_orders(ctx, order_ids),
        ExecuteMsg::ProvideLiquidity { lp_denom } => provide_liquidity(ctx, lp_denom),
        ExecuteMsg::WithdrawLiquidity {} => withdraw_liquidity(ctx),
    }
}

#[inline]
fn batch_update_pairs(ctx: MutableCtx, updates: Vec<PairUpdate>) -> anyhow::Result<Response> {
    let mut events = Vec::with_capacity(updates.len());

    for update in updates {
        PAIRS.save(
            ctx.storage,
            (&update.base_denom, &update.quote_denom),
            &update.params,
        )?;

        events.push(ContractEvent::new(PairUpdated::NAME, PairUpdated {
            base_denom: update.base_denom,
            quote_denom: update.quote_denom,
        })?);
    }

    Ok(Response::new().add_subevents(events))
}

#[inline]
fn create_passive_pool(
    ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
    curve_type: CurveInvariant,
    lp_denom: Denom,
    swap_fee: Udec128,
) -> anyhow::Result<Response> {
    // Only the owner can create a passive pool
    ensure!(
        ctx.sender == ctx.querier.query_owner()?,
        "Only the owner can create a passive pool"
    );

    // Ensure the pool doesn't already exist
    ensure!(
        !POOLS.has(ctx.storage, &lp_denom),
        "Pool already exists for pair ({base_denom}, {quote_denom})"
    );

    // Ensure the LP token denom is valid
    let parts = lp_denom.inner();
    ensure!(
        parts.len() == 3 && parts[0] == *NAMESPACE && parts[1] == *LP_NAMESPACE,
        "invalid LP token denom"
    );

    // Validate swap fee
    ensure!(swap_fee < Udec128::ONE, "swap fee must be less than 100%");

    // Ensure the funds contain only the base and quote denoms and contain both
    ensure!(
        ctx.funds.has(&base_denom) && ctx.funds.has(&quote_denom) && ctx.funds.len() == 2,
        "Invalid funds. Must send only the base and quote denoms and both must be present."
    );

    // Save the LP token denom
    LP_DENOMS.save(ctx.storage, (&base_denom, &quote_denom), &lp_denom)?;

    let (pool, initial_lp_supply) = PassiveLiquidityPool::initialize(
        base_denom,
        quote_denom,
        ctx.funds.try_into()?,
        curve_type,
        swap_fee,
    )?;

    // Create the pool
    POOLS.save(ctx.storage, &lp_denom, &pool)?;

    // Create mint message. Mint the initial LP token supply to the contract
    // to ensure the pool is never emptied.
    let bank = ctx.querier.query_bank()?;
    let mint_msg = Message::execute(
        bank,
        &bank::ExecuteMsg::Mint {
            to: ctx.contract,
            denom: lp_denom,
            amount: initial_lp_supply,
        },
        Coins::new(),
    )?;

    Ok(Response::new().add_message(mint_msg))
}

#[inline]
fn submit_order(
    ctx: MutableCtx,
    base_denom: Denom,
    quote_denom: Denom,
    direction: Direction,
    amount: Uint128,
    price: Udec128,
) -> anyhow::Result<Response> {
    ensure!(
        PAIRS.has(ctx.storage, (&base_denom, &quote_denom)),
        "pair not found with base `{base_denom}` and quote `{quote_denom}`"
    );

    let deposit = ctx.funds.into_one_coin()?;

    match direction {
        Direction::Bid => {
            let amount = amount.checked_mul_dec_ceil(price)?;

            ensure!(
                deposit.denom == quote_denom,
                "incorrect deposit denom for BUY order! expecting: {}, found: {}",
                quote_denom,
                deposit.denom
            );

            ensure!(
                deposit.amount == amount,
                "incorrect deposit amount for BUY order! expecting: {}, found: {}",
                amount,
                deposit.amount
            );
        },
        Direction::Ask => {
            ensure!(
                deposit.denom == base_denom,
                "incorrect deposit denom for SELL order! expecting: {}, found: {}",
                base_denom,
                deposit.denom
            );

            ensure!(
                deposit.amount == amount,
                "incorrect deposit amount for SELL order! expecting: {}, found: {}",
                amount,
                deposit.amount
            );
        },
    }

    let (mut order_id, _) = NEXT_ORDER_ID.increment(ctx.storage)?;

    // For BUY orders, invert the order ID. This is necessary for enforcing
    // price-time priority. See the docs on `OrderId` for details.
    if direction == Direction::Bid {
        order_id = !order_id;
    }

    INCOMING_ORDERS.save(
        ctx.storage,
        (ctx.sender, order_id),
        &(
            (
                (base_denom.clone(), quote_denom.clone()),
                direction,
                price,
                order_id,
            ),
            Order {
                user: ctx.sender,
                amount,
                remaining: amount,
            },
        ),
    )?;

    Ok(Response::new().add_event(OrderSubmitted {
        order_id,
        user: ctx.sender,
        base_denom,
        quote_denom,
        direction,
        price,
        amount,
        deposit,
    })?)
}

#[inline]
fn batch_swap(ctx: MutableCtx, swaps: Vec<Swap>) -> anyhow::Result<Response> {
    let mut funds = ctx.funds.clone();
    for swap in swaps {
        // Read the LP token denom from the storage. If it is not found under
        // either (base_denom, quote_denom) or (quote_denom, base_denom), then
        // error.
        let lp_denom =
            match LP_DENOMS.may_load(ctx.storage, (&swap.base_denom, &swap.quote_denom))? {
                Some(denom) => denom,
                None => LP_DENOMS.load(ctx.storage, (&swap.quote_denom, &swap.base_denom))?,
            };

        // Load the pool
        let mut pool = POOLS.load(ctx.storage, &lp_denom)?;

        // Calculate the out amount and update the pool reserves
        let (offer, ask) = pool.swap(&swap)?;

        // Save the updated pool
        POOLS.save(ctx.storage, &lp_denom, &pool)?;

        // Deduct the offer and add the ask to the funds. The funds sent are mutated
        // by the swap to reflect the user funds after the swap. This allows multiple
        // swaps using the output of the previous swap as the input for the next swap.
        funds
            .deduct(offer)
            .map_err(|_| anyhow::anyhow!("insufficient funds"))?;
        funds.insert(ask)?;
    }

    // Send back any unused funds together with proceeds from swaps
    Ok(Response::new().add_message(Message::transfer(ctx.sender, funds)?))
}

fn cancel_orders(ctx: MutableCtx, order_ids: OrderIds) -> anyhow::Result<Response> {
    let mut refunds = Coins::new();
    let mut events = Vec::new();

    // First, collect all orders to be cancelled into memory.
    let orders = match order_ids {
        // Cancel all orders.
        OrderIds::All => ORDERS
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
        OrderIds::Some(order_ids) => order_ids
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

        events.push(ContractEvent::new("order_canceled", OrderCanceled {
            order_id: *order_id,
            remaining: order.remaining,
            refund: refund.clone(),
        })?);

        refunds.insert(refund)?;

        // Remove the order from storage
        if is_incoming {
            INCOMING_ORDERS.remove(ctx.storage, (ctx.sender, *order_id));
        } else {
            ORDERS.remove(ctx.storage, order_key)?;
        }
    }

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, refunds)?)
        .add_subevents(events))
}

#[inline]
fn provide_liquidity(ctx: MutableCtx, lp_denom: Denom) -> anyhow::Result<Response> {
    // Get the funds from sent
    let funds: CoinPair = ctx.funds.try_into()?;

    let mut pool = POOLS.load(ctx.storage, &lp_denom)?;

    // Ensure the funds are valid. They must only contain the base and quote denoms and must contain both
    ensure!(
        funds.has(&pool.base_denom) || funds.has(&pool.quote_denom),
        "Invalid funds. Must send at least one coin in the pair. Sent: {:?}, {:?}, {:?}",
        funds,
        pool.base_denom,
        pool.quote_denom
    );

    // Ensure the pair is registered
    ensure!(
        PAIRS.has(ctx.storage, (&pool.base_denom, &pool.quote_denom)),
        "Pair not found."
    );

    // Ensure the pool has reserves
    ensure!(
        pool.reserves.first().amount.is_non_zero() && pool.reserves.second().amount.is_non_zero(),
        "Cannot add liquidity to pool with zero reserves"
    );

    // Query the LP token supply
    let lp_supply = ctx.querier.query_supply(lp_denom.clone())?;
    assert!(lp_supply.is_non_zero(), "LP token supply is zero");

    // Calculate the funds to provide and the amount of LP tokens to mint
    let mint_ratio = pool.add_liquidity(funds.clone())?;
    let lp_mint_amount = lp_supply.checked_mul_dec_floor(mint_ratio)?;

    // Apply swap fee to unbalanced provision
    fn abs_diff(a: Uint128, b: Uint128) -> Uint128 {
        if a > b {
            a - b
        } else {
            b - a
        }
    }
    let (a, b, reserves_a, reserves_b) = (
        *funds.first().amount,
        *funds.second().amount,
        *pool.reserves.first().amount,
        *pool.reserves.second().amount,
    );
    let sum_reserves = reserves_a.checked_add(reserves_b)?;
    let avg_reserves = sum_reserves.checked_div(Uint128::new(2))?;
    let fee_rate = Udec128::checked_from_ratio(
        abs_diff(a, avg_reserves).checked_add(abs_diff(b, avg_reserves))?,
        sum_reserves,
    )?
    .checked_mul(
        pool.swap_fee
            .checked_div(Udec128::checked_from_ratio(2, 1)?)?,
    )?;
    let lp_mint_amount_after_fees =
        lp_mint_amount.checked_mul_dec_floor(Udec128::ONE.checked_sub(fee_rate)?)?;

    // Save the updated pool
    POOLS.save(ctx.storage, &lp_denom, &pool)?;

    // Create mint message
    let bank = ctx.querier.query_bank()?;
    let mint_msg = Message::execute(
        bank,
        &bank::ExecuteMsg::Mint {
            to: ctx.sender,
            denom: lp_denom,
            amount: lp_mint_amount_after_fees,
        },
        Coins::new(), // No funds needed for minting
    )?;

    Ok(Response::new().add_message(mint_msg))
}

/// Withdraw liquidity from a pool. The LP tokens must be sent with the message.
/// The underlying assets will be returned to the sender.
#[inline]
fn withdraw_liquidity(ctx: MutableCtx) -> anyhow::Result<Response> {
    let sent_lp_tokens = ctx.funds.clone().into_one_coin()?;

    // Query the LP token supply
    let lp_supply = ctx.querier.query_supply(sent_lp_tokens.denom.clone())?;

    // Load the pool
    let mut pool = POOLS.load(ctx.storage, &sent_lp_tokens.denom)?;

    // Calculate the amount of each asset to return
    let coins_to_return = pool.remove_liquidity(sent_lp_tokens.amount, lp_supply)?;

    // Save the updated pool
    POOLS.save(ctx.storage, &sent_lp_tokens.denom, &pool)?;

    // Create burn message
    let bank = ctx.querier.query_bank()?;
    let burn_msg = Message::execute(
        bank,
        &bank::ExecuteMsg::Burn {
            from: ctx.contract,
            denom: sent_lp_tokens.denom,
            amount: sent_lp_tokens.amount,
        },
        Coins::new(), // No funds needed for burning
    )?;

    // Create transfer message
    let transfer_msg = Message::transfer(ctx.sender, coins_to_return)?;

    Ok(Response::default()
        .add_message(burn_msg)
        .add_message(transfer_msg))
}

/// Match and fill orders using the uniform price auction strategy.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    let mut events = Vec::new();
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

    Ok(Response::new()
        .add_message({
            let bank = ctx.querier.query_bank()?;
            Message::execute(
                bank,
                &bank::ExecuteMsg::BatchTransfer(refunds),
                Coins::new(),
            )?
        })
        .add_subevents(events))
}

#[inline]
fn clear_orders_of_pair(
    storage: &mut dyn Storage,
    base_denom: Denom,
    quote_denom: Denom,
    events: &mut Vec<ContractEvent>,
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

    events.push(ContractEvent::new("orders_matched", OrdersMatched {
        base_denom: base_denom.clone(),
        quote_denom: quote_denom.clone(),
        clearing_price,
        volume,
    })?);

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

        events.push(ContractEvent::new("order_filled", OrderFilled {
            order_id,
            clearing_price,
            filled,
            refund: refund.clone(),
            fee: None,
            cleared,
        })?);

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
