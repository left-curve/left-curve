use {
    crate::{
        fill_orders, match_orders, FillingOutcome, MatchingOutcome, Order, INCOMING_ORDERS,
        LP_DENOMS, NEXT_ORDER_ID, ORDERS, PAIRS, POOLS,
    },
    anyhow::{bail, ensure},
    dango_types::{
        bank,
        dex::{
            CurveInvariant, Direction, ExecuteMsg, InstantiateMsg, OrderCanceled, OrderFilled,
            OrderId, OrderSubmitted, OrdersMatched, PairUpdate, PairUpdated, Pool, Swap,
            LP_NAMESPACE, NAMESPACE,
        },
    },
    grug::{
        Addr, Coin, Coins, ContractEvent, Denom, EventName, Inner, Message, MultiplyFraction,
        MutableCtx, Number, NumberConst, Order as IterationOrder, QuerierExt, Response, StdResult,
        Storage, SudoCtx, Udec128, Uint128,
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

    // Save the LP token denom
    LP_DENOMS.save(ctx.storage, (&base_denom, &quote_denom), &lp_denom)?;

    // Create the pool
    POOLS.save(ctx.storage, &lp_denom, &Pool {
        base_denom,
        quote_denom,
        curve_type,
        reserves: Coins::new(),
        swap_fee,
    })?;

    Ok(Response::new())
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
        order_id,
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
        let (offer, ask) = match swap.direction {
            Direction::Ask => {
                let offer = Coin::new(swap.base_denom, swap.amount)?;
                (
                    offer.clone(),
                    pool.swap_exact_amount_in(offer, &swap.quote_denom)?,
                )
            },
            Direction::Bid => {
                let ask = Coin::new(swap.base_denom, swap.amount)?;
                (
                    pool.swap_exact_amount_out(ask.clone(), &swap.quote_denom)?,
                    ask,
                )
            },
        };

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

#[inline]
fn cancel_orders(ctx: MutableCtx, order_ids: BTreeSet<OrderId>) -> anyhow::Result<Response> {
    let mut refunds = Coins::new();
    let mut events = Vec::new();

    for order_id in order_ids {
        // First try to load from resting orders, then from incoming orders storage.
        // If found in neither or both through an error
        let (((base_denom, quote_denom), direction, price, _), order) = match (
            ORDERS.idx.order_id.may_load(ctx.storage, order_id)?,
            INCOMING_ORDERS.may_load(ctx.storage, order_id)?,
        ) {
            // If the order is in the persistent storage, then remove it from
            // there.
            (Some((order_key, order)), None) => {
                ORDERS.remove(ctx.storage, order_key.clone())?;
                (order_key, order)
            },
            // If the order is in the incoming orders map, then remove it from
            // there. This should be rare, as the trader would need to submit
            // then remove the order in the same block.
            (None, Some((order_key, order))) => {
                INCOMING_ORDERS.remove(ctx.storage, order_id);
                (order_key, order)
            },
            // Order is not found in either place, error.
            (None, None) => {
                bail!("order with id `{order_id}` not found");
            },
            // Order is found in both maps. This should never happen, indicating
            // a serious bug. We panic and halt the chain for investigation.
            (Some(_), Some(_)) => {
                unreachable!("order with id `{order_id}` found in both maps");
            },
        };

        ensure!(
            ctx.sender == order.user,
            "only the user can cancel the order"
        );

        let refund = match direction {
            Direction::Bid => Coin {
                denom: quote_denom.clone(),
                amount: order.remaining.checked_mul_dec_floor(price)?,
            },
            Direction::Ask => Coin {
                denom: base_denom.clone(),
                amount: order.remaining,
            },
        };

        events.push(ContractEvent::new("order_canceled", OrderCanceled {
            order_id,
            remaining: order.remaining,
            refund: refund.clone(),
        })?);

        refunds.insert(refund)?;
    }

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, refunds)?)
        .add_subevents(events))
}

/// Provide liquidity to a pool. The liquidity is provided in the same ratio as the
/// current pool reserves. The required funds MUST be sent with the message. Any
/// funds sent in excess of the required amount will be returned to the sender.
#[inline]
fn provide_liquidity(ctx: MutableCtx, lp_denom: Denom) -> anyhow::Result<Response> {
    // Get the funds from sent
    let funds = &ctx.funds;

    let mut pool = POOLS.load(ctx.storage, &lp_denom)?;

    // Ensure the funds are valid. They must only contain the base and quote denoms and must contain both
    ensure!(
        funds.len() == 2 && funds.has(&pool.base_denom) && funds.has(&pool.quote_denom),
        "Invalid funds. Must send both coins in the pair. {:?}, {:?}",
        funds,
        pool.reserves
    );

    // Ensure the pair is registered
    ensure!(
        PAIRS.has(ctx.storage, (&pool.base_denom, &pool.quote_denom)),
        "Pair not found."
    );

    // Query the LP token supply
    let lp_supply = ctx.querier.query_supply(lp_denom.clone())?;

    // Calculate the funds to provide and the amount of LP tokens to mint
    let (funds_to_provide, lp_mint_amount) = match lp_supply {
        Uint128::ZERO => (
            funds.clone(),
            funds
                .clone()
                .into_iter()
                .fold(Uint128::ONE, |acc, c| acc * c.amount),
        ),
        _ => {
            // Calculate the minimum ratio of sent funds to the pool reserves
            let min_ratio = funds
                .into_iter()
                .map(|c| {
                    Udec128::checked_from_ratio(*c.amount, pool.reserves.amount_of(c.denom))
                        .unwrap()
                })
                .min()
                .unwrap_or(Udec128::ZERO);

            // Ensure the minimum ratio is greater than 0
            ensure!(
                min_ratio > Udec128::ZERO,
                "Invalid funds. Must send both coins in the pair."
            );

            // The funds to use in the provision are the pool reserves scaled by the minimum ratio.
            // This ensures that the provision is made in the same ratio as the pool reserves.
            let funds_to_provide = pool
                .reserves
                .clone()
                .into_iter()
                .map(|c| {
                    Ok(Coin {
                        denom: c.denom.clone(),
                        amount: c.amount.checked_mul_dec_floor(min_ratio)?,
                    })
                })
                .collect::<StdResult<Vec<Coin>>>()?
                .try_into()?;
            let lp_mint_amount = lp_supply.checked_mul_dec_floor(min_ratio)?;

            (funds_to_provide, lp_mint_amount)
        },
    };

    // Calculate funds to provide and funds to return
    let funds_to_return = funds.clone().deduct_many(funds_to_provide.clone())?.clone();

    // Update the pool reserves
    pool.reserves.insert_many(funds_to_provide)?;
    POOLS.save(ctx.storage, &lp_denom, &pool)?;

    // Create mint message
    let bank = ctx.querier.query_bank()?;
    let mint_msg = Message::execute(
        bank,
        &bank::ExecuteMsg::Mint {
            to: ctx.sender,
            denom: lp_denom,
            amount: lp_mint_amount,
        },
        Coins::new(), // No funds needed for minting
    )?;

    Ok(Response::default()
        .add_message(Message::transfer(ctx.sender, funds_to_return)?)
        .add_message(mint_msg))
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
    let ratio = Udec128::checked_from_ratio(sent_lp_tokens.amount, lp_supply)?;
    let coins_to_return: Coins = pool
        .reserves
        .clone()
        .into_iter()
        .map(|c| Coin {
            denom: c.denom.clone(),
            amount: c.amount.checked_mul_dec_floor(ratio).unwrap(),
        })
        .collect::<Vec<Coin>>()
        .try_into()?;

    // Update the pool reserves
    pool.reserves.deduct_many(coins_to_return.clone())?;
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
