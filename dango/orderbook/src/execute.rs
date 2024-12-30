use {
    crate::{Order, NEW_ORDER_COUNTS, NEXT_ORDER_ID, ORDERS},
    anyhow::ensure,
    dango_types::orderbook::{Direction, ExecuteMsg, InstantiateMsg, OrderId},
    grug::{
        Addr, Coin, Coins, Denom, IsZero, Message, MsgTransfer, MultiplyFraction, MutableCtx,
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

    // For BUY orders, we bitwise reverse the `order_id` (which is numerically
    // equivalent to `OrderId::MAX - order_id`). This ensures that when matching
    // orders, the older orders are matched first.
    //
    // Note that this assumes `order_id` never exceeds `u64::MAX / 2`, which is
    // a safe assumption. If we accept 1 million orders per second, it would
    // take ~300,000 years to reach `u64::MAX / 2`.
    if direction == Direction::Bid {
        order_id = !order_id;
    }

    NEW_ORDER_COUNTS.increment(ctx.storage, (&base_denom, &quote_denom))?;

    ORDERS.save(
        ctx.storage,
        ((base_denom, quote_denom), direction, price, order_id),
        &Order {
            trader: ctx.sender,
            amount,
            remaining: amount,
        },
    )?;

    Ok(Response::new())
}

#[inline]
fn cancel_orders(ctx: MutableCtx, order_ids: BTreeSet<OrderId>) -> anyhow::Result<Response> {
    let mut refund = Coins::new();

    for order_id in order_ids {
        let (((base_denom, quote_denom), direction, price, _), order) =
            ORDERS.idx.order_id.load(ctx.storage, order_id)?;

        ensure!(
            ctx.sender == order.trader,
            "only the trader can cancel the order"
        );

        match direction {
            Direction::Bid => {
                refund.insert(Coin {
                    denom: quote_denom.clone(),
                    amount: order.remaining.checked_mul_dec_floor(price)?,
                })?;
            },
            Direction::Ask => {
                refund.insert(Coin {
                    denom: base_denom.clone(),
                    amount: order.remaining,
                })?;
            },
        };

        ORDERS.remove(
            ctx.storage,
            ((base_denom, quote_denom), direction, price, order_id),
        )?;
    }

    Ok(Response::new().add_message(Message::transfer(ctx.sender, refund)?))
}

/// Match and fill orders using the uniform price auction strategy.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    // Tracks how much fund should be transferred from this contract to traders.
    let mut transfers = BTreeMap::<Addr, Coins>::new();

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

        // Drop the iterators, which hold immutable references to `ctx.storage`,
        // so that we can write to storage now.
        drop(bid_iter);
        drop(ask_iter);

        // If no matching orders were found, then we're done with this pair.
        // Continue to the next pair.
        let Some((lower_price, higher_price)) = range else {
            continue;
        };

        // Choose the clearing price. Any price within `range` gives the same
        // volume (measured in the base asset). We can either take the lower end,
        // the higher end, or the midpoint of the range. Here we choose the midpoint.
        let clearing_price = lower_price.checked_add(higher_price)?.checked_mul(HALF)?;

        // The volume of this auction is the smaller between bid volume and ask volume.
        let mut bid_volume = bid_volume.min(ask_volume);
        let mut ask_volume = bid_volume;

        // Clear the BUY orders.
        //
        // Note: if the clearing price is better than the bid price, we need to
        // refund the trader the unused quote asset.
        for ((bid_price, order_id), mut bid_order) in bids {
            let volume = bid_order.remaining.min(bid_volume);

            bid_order.remaining -= volume;
            bid_volume -= volume;

            transfers
                .entry(bid_order.trader)
                .or_default()
                .insert(Coin {
                    denom: base_denom.clone(),
                    amount: volume,
                })?
                .insert(Coin {
                    denom: quote_denom.clone(),
                    amount: volume.checked_mul_dec_floor(bid_price - clearing_price)?,
                })?;

            if bid_order.remaining.is_non_zero() {
                ORDERS.save(
                    ctx.storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        Direction::Bid,
                        bid_price,
                        order_id,
                    ),
                    &bid_order,
                )?;
            } else {
                ORDERS.remove(
                    ctx.storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        Direction::Bid,
                        bid_price,
                        order_id,
                    ),
                )?;
            }

            if bid_volume.is_zero() {
                break;
            }
        }

        // Clear the SELL orders.
        for ((ask_price, order_id), mut ask_order) in asks {
            let volume = ask_order.remaining.min(ask_volume);

            ask_order.remaining -= volume;
            ask_volume -= volume;

            transfers
                .entry(ask_order.trader)
                .or_default()
                .insert(Coin {
                    denom: quote_denom.clone(),
                    amount: volume.checked_mul_dec_floor(clearing_price)?,
                })?;

            if ask_order.remaining.is_non_zero() {
                ORDERS.save(
                    ctx.storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        Direction::Ask,
                        ask_price,
                        order_id,
                    ),
                    &ask_order,
                )?;
            } else {
                ORDERS.remove(
                    ctx.storage,
                    (
                        (base_denom.clone(), quote_denom.clone()),
                        Direction::Ask,
                        ask_price,
                        order_id,
                    ),
                )?;
            }

            if ask_volume.is_zero() {
                break;
            }
        }
    }

    // Reset the order counters for the next block.
    NEW_ORDER_COUNTS.reset_all(ctx.storage);

    // TODO: create a batch send method at bank contract
    let messages = transfers
        .into_iter()
        .map(|(to, coins)| MsgTransfer { to, coins });

    Ok(Response::new().add_messages(messages))
}
