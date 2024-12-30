use {
    crate::{ORDERS, ORDER_ID, PAIR},
    anyhow::ensure,
    dango_types::orderbook::{Direction, ExecuteMsg, InstantiateMsg, Order, OrderId},
    grug::{
        Addr, Coin, Coins, Message, MsgTransfer, MultiplyFraction, MutableCtx, Number, NumberConst,
        Order as IterationOrder, Response, StdResult, SudoCtx, Udec128, Uint128,
    },
    std::collections::{BTreeMap, BTreeSet},
};

// equals 0.5
const HALF: Udec128 = Udec128::raw(Uint128::new(500_000_000_000_000_000));

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

    let (mut order_id, _) = ORDER_ID.increment(ctx.storage)?;

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

    ORDERS.save(ctx.storage, (direction, price, order_id), &Order {
        trader: ctx.sender,
        amount,
        remaining: amount,
    })?;

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

        ORDERS.remove(ctx.storage, (direction, price, order_id))?;
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
    let mut bids = Vec::new();
    let mut bid_is_new = true;
    let mut bid_volume = Uint128::ZERO;
    let mut ask = ask_iter.next().transpose()?;
    let mut asks = Vec::new();
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

    // The volume of this auction is the smaller between bid volume and ask volume.
    let mut bid_volume = bid_volume.min(ask_volume);
    let mut ask_volume = bid_volume;

    // Loop through BUY and SELL orders respectively and clear them.
    // Track how much coins should be refunded to each trader, respectively.
    let mut refunds = BTreeMap::<Addr, Coins>::new();

    // Clear the BUY orders.
    for ((bid_price, order_id), mut bid_order) in bids {
        // Note: if the clearing price is better than the bid price, we need to
        // refund the trader the unused quote asset.
        let price_diff = bid_price - clearing_price;

        if bid_order.remaining <= bid_volume {
            // This order can be fully filled.
            bid_volume -= bid_order.remaining;

            refunds
                .entry(bid_order.trader)
                .or_default()
                .insert(Coin {
                    denom: pair.base_denom.clone(),
                    amount: bid_order.remaining,
                })?
                .insert(Coin {
                    denom: pair.quote_denom.clone(),
                    amount: bid_order.remaining.checked_mul_dec_floor(price_diff)?,
                })?;

            ORDERS.remove(ctx.storage, (Direction::Bid, bid_price, order_id))?;
        } else {
            // This order can only be partially filled.
            bid_order.remaining -= bid_volume;

            refunds
                .entry(bid_order.trader)
                .or_default()
                .insert(Coin {
                    denom: pair.base_denom.clone(),
                    amount: bid_volume,
                })?
                .insert(Coin {
                    denom: pair.quote_denom.clone(),
                    amount: bid_order.remaining.checked_mul_dec_floor(price_diff)?,
                })?;

            ORDERS.save(
                ctx.storage,
                (Direction::Bid, bid_price, order_id),
                &bid_order,
            )?;

            break;
        }
    }

    // Clear the SELL orders.
    for ((ask_price, order_id), mut ask_order) in asks {
        if ask_order.remaining <= ask_volume {
            // This order can be fully filled.
            ask_volume -= ask_order.remaining;

            refunds.entry(ask_order.trader).or_default().insert(Coin {
                denom: pair.quote_denom.clone(),
                amount: ask_order.remaining.checked_mul_dec_floor(clearing_price)?,
            })?;

            ORDERS.remove(ctx.storage, (Direction::Ask, ask_price, order_id))?;
        } else {
            // This order can only be partially filled.
            ask_order.remaining -= ask_volume;

            refunds.entry(ask_order.trader).or_default().insert(Coin {
                denom: pair.quote_denom.clone(),
                amount: ask_volume.checked_mul_dec_floor(clearing_price)?,
            })?;

            ORDERS.save(
                ctx.storage,
                (Direction::Ask, ask_price, order_id),
                &ask_order,
            )?;

            break;
        }
    }

    // TODO: create a batch send method at bank contract
    let messages = refunds
        .into_iter()
        .map(|(to, coins)| MsgTransfer { to, coins });

    Ok(Response::new().add_messages(messages))
}

// ----------------------------------- tests -----------------------------------

// Test cases from:
// https://motokodefi.substack.com/p/uniform-price-call-auctions-a-better
#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::orderbook::Pair,
        grug::{btree_map, Denom, MockContext},
        std::{str::FromStr, sync::LazyLock},
        test_case::test_case,
    };

    static BASE_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("base").unwrap());
    static QUOTE_DENOM: LazyLock<Denom> = LazyLock::new(|| Denom::from_str("quote").unwrap());

    // ------------------------------- example 1 -------------------------------
    #[test_case(
        vec![
            (
                (
                    Direction::Bid,
                    Udec128::new(30),
                    !0,
                ),
                Order {
                    trader: Addr::mock(0),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Bid,
                    Udec128::new(20),
                    !1,
                ),
                Order {
                    trader: Addr::mock(1),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Bid,
                    Udec128::new(10),
                    !2,
                ),
                Order {
                    trader: Addr::mock(2),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(10),
                    3,
                ),
                Order {
                    trader: Addr::mock(3),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(20),
                    4,
                ),
                Order {
                    trader: Addr::mock(4),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(30),
                    5,
                ),
                Order {
                    trader: Addr::mock(5),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
        ],
        vec![
            (
                (
                    Direction::Bid,
                    Udec128::new(10),
                    !2,
                ),
                Order {
                    trader: Addr::mock(2),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(30),
                    5,
                ),
                Order {
                    trader: Addr::mock(5),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
        ],
        vec![
            // Trader created a BUY order of 10 BASE at 30 QUOTE per base with a
            // a deposit of 10 * 30 = 300 QUOTE.
            // The order executed at at 20 QUOTE per base, so trader gets:
            // 10 BASE + refund of 10 * (30 - 20) = 100 QUOTE.
            Message::Transfer(MsgTransfer {
                to: Addr::mock(0),
                coins: Coins::new_unchecked(btree_map! {
                    BASE_DENOM.clone() => Uint128::new(10),
                    QUOTE_DENOM.clone() => Uint128::new(100),
                }),
            }),
            Message::Transfer(MsgTransfer {
                to: Addr::mock(1),
                coins: Coins::new_unchecked(btree_map! {
                    BASE_DENOM.clone() => Uint128::new(10),
                }),
            }),
            // Trader created a SELL order of 10 BASE at 10 QUOTE per base with
            // a deposit of 10 BASE.
            // The order executed at 20 QUOTE per base, so trader gets:
            // 10 * 20 = 200 QUOTE.
            Message::Transfer(MsgTransfer {
                to: Addr::mock(3),
                coins: Coins::new_unchecked(btree_map! {
                    QUOTE_DENOM.clone() => Uint128::new(200),
                }),
            }),
            Message::Transfer(MsgTransfer {
                to: Addr::mock(4),
                coins: Coins::new_unchecked(btree_map! {
                    QUOTE_DENOM.clone() => Uint128::new(200),
                }),
            }),
        ];
        "example 1"
    )]
    // ------------------------------- example 2 -------------------------------
    #[test_case(
        vec![
            (
                (
                    Direction::Bid,
                    Udec128::new(30),
                    !0,
                ),
                Order {
                    trader: Addr::mock(0),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Bid,
                    Udec128::new(20),
                    !1,
                ),
                Order {
                    trader: Addr::mock(1),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Bid,
                    Udec128::new(10),
                    !2,
                ),
                Order {
                    trader: Addr::mock(2),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            // Compared to the last example, this time each seller quotes their
            // price 5 QUOTE lower. As a result, the clearing price is lowered
            // from 20 to 17.5 QUOTE per BASE.
            (
                (
                    Direction::Ask,
                    Udec128::new(5),
                    3,
                ),
                Order {
                    trader: Addr::mock(3),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(15),
                    4,
                ),
                Order {
                    trader: Addr::mock(4),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(25),
                    5,
                ),
                Order {
                    trader: Addr::mock(5),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
        ],
        vec![
            (
                (
                    Direction::Bid,
                    Udec128::new(10),
                    !2,
                ),
                Order {
                    trader: Addr::mock(2),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(25),
                    5,
                ),
                Order {
                    trader: Addr::mock(5),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
        ],
        vec![
            // Trader created a BUY order of 10 BASE at 30 QUOTE per base with a
            // Trader created a BUY order of 10 BASE at 30 QUOTE per base
            // with a deposit of 10 * 30 = 300 QUOTE.
            // The order executed at at 17.5 QUOTE per base, so trader gets
            // 10 BASE + refund of 10 * (30 - 17.5) = 125 QUOTE.
            Message::Transfer(MsgTransfer {
                to: Addr::mock(0),
                coins: Coins::new_unchecked(btree_map! {
                    BASE_DENOM.clone() => Uint128::new(10),
                    QUOTE_DENOM.clone() => Uint128::new(125),
                }),
            }),
            // Trader created a BUY order of 10 BASE at 30 QUOTE per base with a
            // deposit of 10 * 20 = 200 QUOTE.
            // The order executed at at 17.5 QUOTE per base, so trader gets
            // 10 BASE + refund of 10 * (20 - 17.5) = 125 QUOTE.
            Message::Transfer(MsgTransfer {
                to: Addr::mock(1),
                coins: Coins::new_unchecked(btree_map! {
                    BASE_DENOM.clone() => Uint128::new(10),
                    QUOTE_DENOM.clone() => Uint128::new(25),
                }),
            }),
            // Traders 3 and 4 both gets their SELL orders completely filled at
            // 17.5 QUOTE per BASE, so they each get 10 * 17.5 = 175 QUOTE.
            Message::Transfer(MsgTransfer {
                to: Addr::mock(3),
                coins: Coins::new_unchecked(btree_map! {
                    QUOTE_DENOM.clone() => Uint128::new(175),
                }),
            }),
            Message::Transfer(MsgTransfer {
                to: Addr::mock(4),
                coins: Coins::new_unchecked(btree_map! {
                    QUOTE_DENOM.clone() => Uint128::new(175),
                }),
            }),
        ];
        "example 2"
    )]
    // ------------------------------- example 3 -------------------------------
    #[test_case(
        vec![
            (
                (
                    Direction::Bid,
                    Udec128::new(30),
                    !0,
                ),
                Order {
                    trader: Addr::mock(0),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Bid,
                    Udec128::new(20),
                    !1,
                ),
                Order {
                    trader: Addr::mock(1),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Bid,
                    Udec128::new(10),
                    !2,
                ),
                Order {
                    trader: Addr::mock(2),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(5),
                    3,
                ),
                Order {
                    trader: Addr::mock(3),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(15),
                    4,
                ),
                Order {
                    trader: Addr::mock(4),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(25),
                    5,
                ),
                Order {
                    trader: Addr::mock(5),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            // Compare to the last example, this time we have this additional
            // BUY order. It has a better price than order `!2`, so `!2` only
            // gets partially filled this time.
            (
                (
                    Direction::Bid,
                    Udec128::new(30),
                    !6,
                ),
                Order {
                    trader: Addr::mock(6),
                    amount: Uint128::new(5),
                    remaining: Uint128::new(5),
                },
            ),
        ],
        vec![
            (
                (
                    Direction::Bid,
                    Udec128::new(10),
                    !2,
                ),
                Order {
                    trader: Addr::mock(2),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
            // This time, this order is only half filled, so it stays in the book.
            (
                (
                    Direction::Bid,
                    Udec128::new(20),
                    !1,
                ),
                Order {
                    trader: Addr::mock(1),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(5),
                },
            ),
            (
                (
                    Direction::Ask,
                    Udec128::new(25),
                    5,
                ),
                Order {
                    trader: Addr::mock(5),
                    amount: Uint128::new(10),
                    remaining: Uint128::new(10),
                },
            ),
        ],
        vec![
            Message::Transfer(MsgTransfer {
                to: Addr::mock(0),
                coins: Coins::new_unchecked(btree_map! {
                    BASE_DENOM.clone() => Uint128::new(10),
                    QUOTE_DENOM.clone() => Uint128::new(125),
                }),
            }),
            // Only 5 BASE is filled, so refund: 5 * (20 - 17.5) = 12, rounded
            // down to 12 QUOTE.
            Message::Transfer(MsgTransfer {
                to: Addr::mock(1),
                coins: Coins::new_unchecked(btree_map! {
                    BASE_DENOM.clone() => Uint128::new(5),
                    QUOTE_DENOM.clone() => Uint128::new(12),
                }),
            }),
            Message::Transfer(MsgTransfer {
                to: Addr::mock(3),
                coins: Coins::new_unchecked(btree_map! {
                    QUOTE_DENOM.clone() => Uint128::new(175),
                }),
            }),
            Message::Transfer(MsgTransfer {
                to: Addr::mock(4),
                coins: Coins::new_unchecked(btree_map! {
                    QUOTE_DENOM.clone() => Uint128::new(175),
                }),
            }),
            // Trader created a BUY order of 5 BASE at 30 QUOTE per base with a
            // deposit of 5 * 30 = 150 QUOTE.
            // The order executed at at 17.5 QUOTE per base, so trader gets:
            // 5 BASE + refund of 5 * (30 - 17.5) = 62.5 QUOTE, rounded down to
            // 62 QUOTE.
            Message::Transfer(MsgTransfer {
                to: Addr::mock(6),
                coins: Coins::new_unchecked(btree_map! {
                    BASE_DENOM.clone() => Uint128::new(5),
                    QUOTE_DENOM.clone() => Uint128::new(62),
                }),
            }),
        ];
        "example 3"
    )]
    fn clear_orders_works(
        before_orders: Vec<((Direction, Udec128, OrderId), Order)>,
        after_orders: Vec<((Direction, Udec128, OrderId), Order)>,
        refunds: Vec<Message>,
    ) {
        let mut ctx = MockContext::new();

        PAIR.save(&mut ctx.storage, &Pair {
            base_denom: BASE_DENOM.clone(),
            quote_denom: QUOTE_DENOM.clone(),
        })
        .unwrap();

        for (key, order) in before_orders {
            ORDERS.save(&mut ctx.storage, key, &order).unwrap();
        }

        let messages = cron_execute(ctx.as_sudo())
            .unwrap()
            .submsgs
            .into_iter()
            .map(|submsg| submsg.msg)
            .collect::<Vec<_>>();
        assert_eq!(messages, refunds);

        let orders = ORDERS
            .range(&ctx.storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(orders, after_orders);
    }
}
