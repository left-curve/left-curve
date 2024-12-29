use {
    crate::{ORDERS, ORDER_ID, PAIR},
    anyhow::ensure,
    dango_types::orderbook::{Direction, ExecuteMsg, InstantiateMsg, Order, OrderId, OrderKey},
    grug::{
        Addr, Coin, Coins, IsZero, Message, MsgTransfer, MultiplyFraction, MutableCtx, Number,
        NumberConst, Order as IterationOrder, Response, StdResult, SudoCtx, Udec128, Uint128,
    },
    std::collections::{BTreeMap, BTreeSet},
};

const HALF: Udec128 = Udec128::new(500_000_000_000_000_000);

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn instantiate(ctx: MutableCtx, msg: InstantiateMsg) -> StdResult<Response> {
    PAIR.save(ctx.storage, &msg.pair)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), grug::export)]
pub fn execute(ctx: MutableCtx, msg: ExecuteMsg) -> anyhow::Result<Response> {
    match msg {
        ExecuteMsg::SubmitOrder {
            direction,
            amount,
            price,
        } => submit_order(ctx, direction, amount, price),
        ExecuteMsg::CancelOrders { order_ids } => cancel_orders(ctx, order_ids),
    }
}

#[inline]
fn submit_order(
    ctx: MutableCtx,
    direction: Direction,
    amount: Uint128,
    price: Udec128,
) -> anyhow::Result<Response> {
    let pair = PAIR.load(ctx.storage)?;
    let deposit = ctx.funds.into_one_coin()?;

    match direction {
        Direction::Bid => {
            let amount = amount.checked_mul_dec_ceil(price)?;

            ensure!(
                deposit.denom == pair.quote_denom,
                "incorrect deposit denom for BUY order! expecting: {}, found: {}",
                pair.quote_denom,
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
                deposit.denom == pair.base_denom,
                "incorrect deposit denom for SELL order! expecting: {}, found: {}",
                pair.base_denom,
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

    let (order_id, _) = ORDER_ID.increment(ctx.storage)?;

    ORDERS.save(
        ctx.storage,
        OrderKey {
            direction,
            price,
            order_id,
        },
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
    let pair = PAIR.load(ctx.storage)?;
    let mut refund = Coins::new();

    for order_id in order_ids {
        let ((direction, price, _), order) = ORDERS.idx.order_id.load(ctx.storage, order_id)?;

        ensure!(
            ctx.sender == order.trader,
            "only the trader can cancel the order"
        );

        match direction {
            Direction::Bid => {
                let amount = order.remaining.checked_mul_dec_floor(price)?;
                refund.insert(Coin {
                    denom: pair.quote_denom.clone(),
                    amount,
                })?;
            },
            Direction::Ask => {
                refund.insert(Coin {
                    denom: pair.base_denom.clone(),
                    amount: order.remaining,
                })?;
            },
        };

        ORDERS.remove(ctx.storage, OrderKey {
            direction,
            price,
            order_id,
        })?;
    }

    Ok(Response::new().add_message(Message::transfer(ctx.sender, refund)?))
}

/// Execute matching orders, return the list of assets to be sent back to
/// traders whose orders have been filled.
///
/// Implemented according to:
/// <https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better>
#[cfg_attr(not(feature = "library"), grug::export)]
pub fn cron_execute(ctx: SudoCtx) -> StdResult<Response> {
    let pair = PAIR.load(ctx.storage)?;

    // Iterate BUY orders from the highest price to the lowest.
    // For orders of the same price, the older one (smaller `order_id`) first.
    let mut bid_iter =
        ORDERS
            .prefix(Direction::Bid)
            .range(ctx.storage, None, None, IterationOrder::Descending);

    // Iterate SELL orders from the lowest price to the highest.
    let mut ask_iter =
        ORDERS
            .prefix(Direction::Ask)
            .range(ctx.storage, None, None, IterationOrder::Ascending);

    // Loop through the orders to find:
    // - the price range that maximizes the volume of trades;
    // - the orders that can be cleared in this price range.
    let mut bid = bid_iter.next().transpose()?;
    let mut bids = BTreeMap::new();
    let mut bid_is_new = true;
    let mut bid_volume = Uint128::ZERO;
    let mut ask = ask_iter.next().transpose()?;
    let mut asks = BTreeMap::new();
    let mut ask_is_new = true;
    let mut ask_volume = Uint128::ZERO;
    let mut range = None;

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
            bids.insert((bid_price, bid_order_id), bid_order);
            bid_volume.checked_add_assign(bid_order.remaining)?;
        }

        if ask_is_new {
            asks.insert((ask_price, ask_order_id), ask_order);
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

    // Drop the iterators, which hold immutable references to `ctx.storage`, so
    // that we can write to storage now.
    drop(bid_iter);
    drop(ask_iter);

    // If no matching orders were found, return early.
    let Some((lower_price, higher_price)) = range else {
        return Ok(Response::new());
    };

    // Choose the clearing price. Any price within `range` gives the same volume
    // (measured in the base asset). We can either take the lower end, the higher
    // end, or the midpoint of the range. Here we choose the midpoint.
    let clearing_price = lower_price.checked_add(higher_price)?.checked_mul(HALF)?;

    // Loop through the orders again and clear them.
    let mut bid_iter = bids.into_iter();
    let mut bid = bid_iter.next();
    let mut ask_iter = asks.into_iter();
    let mut ask = ask_iter.next();
    let mut sends = BTreeMap::<Addr, Coins>::new();

    loop {
        let Some(((bid_price, bid_order_id), bid_order)) = &mut bid else {
            break;
        };

        let Some(((ask_price, ask_order_id), ask_order)) = &mut ask else {
            break;
        };

        debug_assert!(
            clearing_price <= *bid_price,
            "bid price should be smaller than clearing price, but got clearing price: {clearing_price}, bid price: {bid_price}"
        );

        debug_assert!(
            clearing_price >= *ask_price,
            "ask price should be greater than clearing price, but got clearing price: {clearing_price}, ask price: {ask_price}"
        );

        let filled = bid_order.remaining.min(ask_order.remaining);

        bid_order.remaining -= filled;
        ask_order.remaining -= filled;

        #[cfg(debug_assertions)]
        {
            bid_volume -= filled;
            ask_volume -= filled;
        }

        sends.entry(bid_order.trader).or_default().insert(Coin {
            denom: pair.base_denom.clone(),
            amount: filled,
        })?;

        sends.entry(ask_order.trader).or_default().insert(Coin {
            denom: pair.quote_denom.clone(),
            amount: filled.checked_mul_dec_floor(clearing_price)?,
        })?;

        if bid_order.remaining.is_zero() {
            ORDERS.remove(ctx.storage, OrderKey {
                direction: Direction::Bid,
                price: *bid_price,
                order_id: *bid_order_id,
            })?;

            bid = bid_iter.next();
        }

        if ask_order.remaining.is_zero() {
            ORDERS.remove(ctx.storage, OrderKey {
                direction: Direction::Ask,
                price: *ask_price,
                order_id: *ask_order_id,
            })?;

            ask = ask_iter.next();
        }
    }

    if let Some(((bid_price, bid_order_id), bid_order)) = bid {
        if bid_order.remaining.is_non_zero() {
            ORDERS.save(
                ctx.storage,
                OrderKey {
                    direction: Direction::Bid,
                    price: bid_price,
                    order_id: bid_order_id,
                },
                &bid_order,
            )?;
        }
    }

    if let Some(((ask_price, ask_order_id), ask_order)) = ask {
        if ask_order.remaining.is_non_zero() {
            ORDERS.save(
                ctx.storage,
                OrderKey {
                    direction: Direction::Ask,
                    price: ask_price,
                    order_id: ask_order_id,
                },
                &ask_order,
            )?;
        }
    }

    debug_assert!(
        bid_volume.is_zero() || ask_volume.is_zero(),
        "one of bid or ask volume should have been reduced to zero, but got bid volume: {bid_volume}, ask volume: {ask_volume}"
    );

    // TODO: create a batch send method at bank contract
    let messages = sends
        .into_iter()
        .map(|(to, coins)| MsgTransfer { to, coins });

    Ok(Response::new().add_messages(messages))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug::MockStorage, test_case::test_case};

    // Test cases from:
    // https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better
    #[test_case(
        [
            (Direction::Bid, Uint128::new(30), Uint128::new(10)),
            (Direction::Bid, Uint128::new(20), Uint128::new(10)),
            (Direction::Bid, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(20), Uint128::new(10)),
            (Direction::Ask, Uint128::new(30), Uint128::new(10)),
        ],
        ClearOrderOutcome {
            range: Some((
                Uint128::new(20).checked_into_dec().unwrap(),
                Uint128::new(20).checked_into_dec().unwrap()
            )),
            volume: Uint128::new(20),
        };
        "example_one"
    )]
    #[test_case(
        [
            (Direction::Bid, Uint128::new(30), Uint128::new(10)),
            (Direction::Bid, Uint128::new(20), Uint128::new(10)),
            (Direction::Bid, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(5), Uint128::new(10)),
            (Direction::Ask, Uint128::new(15), Uint128::new(10)),
            (Direction::Ask, Uint128::new(25), Uint128::new(10)),
        ],
        ClearOrderOutcome {
            range: Some((
                Uint128::new(15).checked_into_dec().unwrap(),
                Uint128::new(20).checked_into_dec().unwrap()
            )),
            volume: Uint128::new(20),
        };
        "example_two"
    )]
    #[test_case(
        [
            (Direction::Bid, Uint128::new(30), Uint128::new(10)),
            (Direction::Bid, Uint128::new(30), Uint128::new(5)),
            (Direction::Bid, Uint128::new(20), Uint128::new(10)),
            (Direction::Bid, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(5), Uint128::new(10)),
            (Direction::Ask, Uint128::new(15), Uint128::new(10)),
            (Direction::Ask, Uint128::new(25), Uint128::new(10)),
        ],
        ClearOrderOutcome {
            range: Some((
                Uint128::new(15).checked_into_dec().unwrap(),
                Uint128::new(20).checked_into_dec().unwrap()
            )),
            volume: Uint128::new(20),
        };
        "example_three"
    )]
    #[test_case(
        [
            (Direction::Bid, Uint128::new(30), Uint128::new(20)),
            (Direction::Bid, Uint128::new(20), Uint128::new(10)),
            (Direction::Bid, Uint128::new(10), Uint128::new(10)),
            (Direction::Ask, Uint128::new(5), Uint128::new(10)),
            (Direction::Ask, Uint128::new(15), Uint128::new(10)),
            (Direction::Ask, Uint128::new(25), Uint128::new(10)),
        ],
        ClearOrderOutcome {
            range: Some((
                Uint128::new(15).checked_into_dec().unwrap(),
                Uint128::new(30).checked_into_dec().unwrap()
            )),
            volume: Uint128::new(20),
        };
        "example_four"
    )]
    fn clear_orders_works<const N: usize>(
        orders: [(Direction, Uint128, Uint128); N],
        expected: ClearOrderOutcome,
    ) {
        let mut storage = MockStorage::new();

        for (order_id, (direction, price, amount)) in orders.into_iter().enumerate() {
            ORDERS
                .save(
                    &mut storage,
                    OrderKey {
                        direction,
                        price: price.checked_into_dec().unwrap(),
                        order_id: order_id as OrderId,
                    },
                    &Order {
                        trader: Addr::mock(0),
                        amount,
                        remaining: amount,
                    },
                )
                .unwrap();
        }

        let outcome = clear_orders(&storage).unwrap();
        assert_eq!(outcome, expected);
    }
}
