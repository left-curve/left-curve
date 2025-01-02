use {
    crate::{Order, NEW_ORDER_COUNTS, NEXT_ORDER_ID, ORDERS},
    anyhow::ensure,
    dango_types::{
        bank,
        orderbook::{
            Direction, ExecuteMsg, InstantiateMsg, OrderCanceled, OrderFilled, OrderId,
            OrderSubmitted, OrdersMatched,
        },
    },
    grug::{
        Addr, Coin, Coins, ContractEvent, Denom, IsZero, Message, MultiplyFraction, MutableCtx,
        Number, NumberConst, Order as IterationOrder, Response, StdResult, SudoCtx, Udec128,
        Uint128,
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
    let mut refunds = BTreeMap::<Addr, Coins>::new();
    let mut events = Vec::new();

    // Find all pairs that have received new orders during the block.
    let pairs = NEW_ORDER_COUNTS
        .current_range(ctx.storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let (pair, _) = res?;
            Ok(pair)
        })
        .collect::<StdResult<Vec<_>>>()?;

    // Loop through the pairs, match and clear the orders for each of them.
    for (base_denom, quote_denom) in pairs {
        // Iterate BUY orders from the highest price to the lowest.
        // Iterate SELL orders from the lowest price to the highest.
        let mut bid_iter = ORDERS
            .prefix((base_denom.clone(), quote_denom.clone()))
            .append(Direction::Bid)
            .range(ctx.storage, None, None, IterationOrder::Descending);
        let mut ask_iter = ORDERS
            .prefix((base_denom.clone(), quote_denom.clone()))
            .append(Direction::Ask)
            .range(ctx.storage, None, None, IterationOrder::Ascending);

        let mut bid = bid_iter.next().transpose()?;
        let mut bids = Vec::new();
        let mut bid_is_new = true;
        let mut bid_volume = Uint128::ZERO;
        let mut ask = ask_iter.next().transpose()?;
        let mut asks = Vec::new();
        let mut ask_is_new = true;
        let mut ask_volume = Uint128::ZERO;
        let mut range = None;

        // Loop through the orders to find:
        // 1. the price range that maximizes the volume of trades;
        // 2. the orders that can be cleared in this price range.
        loop {
            let Some(((bid_price, bid_order_id), bid_order)) = bid else {
                break;
            };

            let Some(((ask_price, ask_order_id), ask_order)) = ask else {
                break;
            };

            if bid_price < ask_price {
                break;
            }

            range = Some((ask_price, bid_price));

            if bid_is_new {
                bids.push(((bid_price, bid_order_id), bid_order));
                bid_volume.checked_add_assign(bid_order.remaining)?;
            }

            if ask_is_new {
                asks.push(((ask_price, ask_order_id), ask_order));
                ask_volume.checked_add_assign(ask_order.remaining)?;
            }

            if bid_volume <= ask_volume {
                bid = bid_iter.next().transpose()?;
                bid_is_new = true;
            } else {
                bid_is_new = false;
            }

            if ask_volume <= bid_volume {
                ask = ask_iter.next().transpose()?;
                ask_is_new = true;
            } else {
                ask_is_new = false;
            }
        }

        // Drop the iterators.
        // Next we need to make state changes, which requires `&mut ctx.storage`.
        // The iterators hold immutable references `&ctx.storage`, so must be dropped.
        drop(bid_iter);
        drop(ask_iter);

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

        // The volume of this auction is the smaller between bid and ask volumes.
        let volume = bid_volume.min(ask_volume);

        events.push(ContractEvent::new("orders_matched", OrdersMatched {
            base_denom: base_denom.clone(),
            quote_denom: quote_denom.clone(),
            clearing_price,
            volume,
        })?);

        // Clear the BUY orders.
        //
        // Note: if the clearing price is better than the bid price, we need to
        // refund the user the unused quote asset.
        let mut remaining_volume = volume;
        for ((price, order_id), mut order) in bids {
            let filled = order.remaining.min(remaining_volume);

            order.remaining -= filled;
            remaining_volume -= filled;

            let cleared = order.remaining.is_zero();

            let mut refund = Coins::new();

            refund
                .insert(Coin {
                    denom: base_denom.clone(),
                    amount: filled,
                })?
                .insert(Coin {
                    denom: quote_denom.clone(),
                    amount: filled.checked_mul_dec_floor(price - clearing_price)?, // this can be zero
                })?;

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
                        Direction::Bid,
                        price,
                        order_id,
                    ),
                )?;
            } else {
                ORDERS.save(
                    ctx.storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        Direction::Bid,
                        price,
                        order_id,
                    ),
                    &order,
                )?;
            }

            if remaining_volume.is_zero() {
                break;
            }
        }

        // Clear the SELL orders.
        let mut remaining_volume = volume;
        for ((price, order_id), mut order) in asks {
            let filled = order.remaining.min(remaining_volume);

            order.remaining -= filled;
            remaining_volume -= filled;

            let cleared = order.remaining.is_zero();

            let refund = Coin {
                denom: quote_denom.clone(),
                amount: filled.checked_mul_dec_floor(clearing_price)?,
            };

            events.push(ContractEvent::new("order_filled", OrderFilled {
                order_id,
                clearing_price,
                filled,
                refund: refund.clone().into(),
                fee: None,
                cleared,
            })?);

            refunds.entry(order.user).or_default().insert(refund)?;

            if cleared {
                ORDERS.remove(
                    ctx.storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        Direction::Ask,
                        price,
                        order_id,
                    ),
                )?;
            } else {
                ORDERS.save(
                    ctx.storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        Direction::Ask,
                        price,
                        order_id,
                    ),
                    &order,
                )?;
            }

            if remaining_volume.is_zero() {
                break;
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
