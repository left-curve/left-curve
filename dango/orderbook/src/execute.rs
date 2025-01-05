use {
    crate::{
        fill_orders, match_orders, FillingOutcome, MatchingOutcome, Order, NEW_ORDER_COUNTS,
        NEXT_ORDER_ID, ORDERS,
    },
    anyhow::ensure,
    dango_types::{
        bank,
        orderbook::{
            Direction, ExecuteMsg, InstantiateMsg, OrderCanceled, OrderFilled, OrderId,
            OrderSubmitted, OrdersMatched,
        },
    },
    grug::{
        Addr, Coin, Coins, ContractEvent, Denom, Message, MultiplyFraction, MutableCtx, Number,
        Order as IterationOrder, Response, StdResult, SudoCtx, Udec128, Uint128,
    },
    std::collections::{BTreeMap, BTreeSet},
};

// equals 0.5
const HALF: Udec128 = Udec128::raw(Uint128::new(500_000_000_000_000_000));

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(_ctx: MutableCtx, _msg: InstantiateMsg) -> StdResult<Response> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::SubmitOrder {
            base_denom,
            quote_denom,
            direction,
            amount,
            price,
        } => submit_order(ctx, base_denom, quote_denom, direction, amount, price),
        ExecuteMsg::CancelOrders { order_ids } => cancel_orders(ctx, order_ids),
    }
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

    NEW_ORDER_COUNTS.increment(ctx.storage, (&base_denom, &quote_denom))?;

    ORDERS.save(
        ctx.storage,
        (
            (base_denom.clone(), quote_denom.clone()),
            direction,
            price,
            order_id,
        ),
        &Order {
            user: ctx.sender,
            amount,
            remaining: amount,
        },
    )?;

    Ok(
        Response::new().add_event("order_submitted", OrderSubmitted {
            order_id,
            user: ctx.sender,
            base_denom,
            quote_denom,
            direction,
            price,
            amount,
            deposit,
        })?,
    )
}

#[inline]
fn cancel_orders(ctx: MutableCtx, order_ids: BTreeSet<OrderId>) -> anyhow::Result<Response> {
    let mut refunds = Coins::new();
    let mut events = Vec::new();

    for order_id in order_ids {
        let (((base_denom, quote_denom), direction, price, _), order) =
            ORDERS.idx.order_id.load(ctx.storage, order_id)?;

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

        ORDERS.remove(
            ctx.storage,
            ((base_denom, quote_denom), direction, price, order_id),
        )?;
    }

    Ok(Response::new()
        .add_message(Message::transfer(ctx.sender, refunds)?)
        .add_subevents(events))
}

/// Match and fill orders using the uniform price auction strategy.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    let mut events = Vec::new();
    let mut refunds = BTreeMap::<Addr, Coins>::new();

    // Find all pairs that have received new orders during the block.
    let pairs = NEW_ORDER_COUNTS
        .current_range(ctx.storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let (pair, _) = res?;
            Ok(pair)
        })
        .collect::<StdResult<Vec<_>>>()?;

    // Loop through the pairs, match and clear the orders for each of them.
    //
    // TODO: spawn a thread for each pair to process them in parallel.
    for (base_denom, quote_denom) in pairs {
        // Iterate BUY orders from the highest price to the lowest.
        // Iterate SELL orders from the lowest price to the highest.
        let bid_iter = ORDERS
            .prefix((base_denom.clone(), quote_denom.clone()))
            .append(Direction::Bid)
            .range(ctx.storage, None, None, IterationOrder::Descending);
        let ask_iter = ORDERS
            .prefix((base_denom.clone(), quote_denom.clone()))
            .append(Direction::Ask)
            .range(ctx.storage, None, None, IterationOrder::Ascending);

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
            continue;
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
                    ctx.storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        order_direction,
                        order_price,
                        order_id,
                    ),
                )?;
            } else {
                ORDERS.save(
                    ctx.storage,
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
    }

    // Reset the order counters for the next block.
    NEW_ORDER_COUNTS.reset_all(ctx.storage);

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
