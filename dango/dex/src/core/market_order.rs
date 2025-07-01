use {
    crate::{ExtendedOrderId, FillingOutcome, MarketOrder, Order, OrderTrait},
    dango_types::dex::{Direction, OrderId},
    grug::{
        IsZero, MathResult, MultiplyFraction, Number, NumberConst, StdResult, Udec128, Uint128,
    },
    std::{
        cmp::{self},
        collections::HashMap,
        iter::Peekable,
    },
};

/// Match a series of market BUY orders against a series of limit SELL orders.
///
/// ## Returns
///
/// - Filling outcomes for the orders (market and limit).
/// - If there's a limit order that has not been fully filled, returns this order.
pub fn match_market_bids_with_limit_asks<M, L>(
    market_orders: &mut M,
    limit_orders: &mut Peekable<L>,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    current_block_height: u64,
) -> anyhow::Result<(
    HashMap<ExtendedOrderId, FillingOutcome>,
    Option<(Udec128, Order)>,
)>
where
    M: Iterator<Item = (OrderId, MarketOrder)>,
    L: Iterator<Item = StdResult<(Udec128, Order)>>,
{
    let mut market_order_id;
    let mut market_order;
    let mut market_base_bought = Uint128::ZERO;
    let mut market_quote_sold = Uint128::ZERO;
    let mut limit_price;
    let mut limit_order;
    let mut limit_base_sold = Uint128::ZERO;
    let mut limit_quote_bought = Uint128::ZERO;
    let mut max_price;
    let mut outcomes = HashMap::new();

    // Get the first market order.
    match market_orders.next() {
        Some(v) => (market_order_id, market_order) = v,
        None => return Ok((outcomes, None)),
    }

    // Get the first limit order.
    match limit_orders.next() {
        Some(Ok(v)) => (limit_price, limit_order) = v,
        Some(Err(e)) => return Err(e.clone().into()),
        None => return Ok((outcomes, None)),
    }

    // Compute the maximum average execution price for the first market order.
    let one_add_max_slippage = Udec128::ONE.saturating_add(market_order.max_slippage);
    max_price = limit_price.saturating_mul(one_add_max_slippage);

    // Loop through the market and limit order iterators simultaneously.
    loop {
        // Without considering the max slippage, the amount we can fill now is
        // either:
        // - the remaining amount in the market order; or
        // - the amount the market order's remaining budget can affort to buy,
        //   given the limit order's price;
        // whichever is smaller.
        let mut fill_amount = cmp::min(
            *limit_order.remaining(),
            market_order.remaining.checked_div_dec_floor(limit_price)?,
        );

        // Now consider the max slippage:
        // - If the limit order's price is better (lower) or equal to the max
        //   price, then do nothing; we fill as much as possible.
        // - Otherwise, compute the maximum amount we can fill such that the
        //   market order's average execution price doesn't exceed the max price.
        if limit_price > max_price {
            let max_fillable_amount = market_base_bought
                .checked_mul_dec_floor(max_price)?
                .checked_sub(market_quote_sold)?
                .checked_div_dec_floor(limit_price - max_price)?;

            fill_amount = cmp::min(fill_amount, max_fillable_amount);
        }

        let fill_amount_in_quote = fill_amount.checked_mul_dec_ceil(limit_price)?;

        // Update the market order.
        market_order.fill(fill_amount_in_quote)?;
        market_base_bought.checked_add_assign(fill_amount)?;
        market_quote_sold.checked_add_assign(fill_amount_in_quote)?;

        // Update the limit order.
        limit_order.fill(fill_amount)?;
        limit_base_sold.checked_add_assign(fill_amount)?;
        limit_quote_bought.checked_add_assign(fill_amount_in_quote)?;

        // Update the market order's filling outcome.
        outcomes.insert(
            ExtendedOrderId::User(market_order_id),
            new_market_bid_filling_outcome(
                market_order,
                market_base_bought,
                market_quote_sold,
                taker_fee_rate, // Market orders are always takers.
            )?,
        );

        // Update the limit order's filling outcome.
        outcomes.insert(
            limit_order.extended_id(),
            new_limit_ask_filling_outcome(
                limit_order,
                limit_base_sold,
                limit_quote_bought,
                limit_order_fee_rate(
                    &limit_order,
                    current_block_height,
                    maker_fee_rate,
                    taker_fee_rate,
                ),
            )?,
        );

        // Determine whether to move on to the next market order.
        // There are two situations:
        // 1. the market order is fully filled;
        // 2. the market order isn't fully filled, neither is the limit order.
        //    This happens when we can't fill the market order any further
        //    without exceeding the maximum average execution price.
        // Also, compute the max price for this new order.
        if market_order.remaining.is_zero() || limit_order.remaining().is_non_zero() {
            // Advance to the next market order.
            match market_orders.next() {
                Some(v) => {
                    (market_order_id, market_order) = v;
                    market_base_bought = Uint128::ZERO;
                    market_quote_sold = Uint128::ZERO;
                },
                None => break,
            }

            // Compute the maximum average execution price for this market order.
            // If the current limit order isn't fully filled, then use its price
            // as the best price; otherwise, use the next limit order's price.
            let one_add_max_slippage = Udec128::ONE.saturating_add(market_order.max_slippage);
            if limit_order.remaining().is_non_zero() {
                max_price = limit_price.saturating_mul(one_add_max_slippage);
            } else {
                match limit_orders.peek() {
                    Some(Ok((price, _))) => {
                        max_price = price.saturating_mul(one_add_max_slippage);
                    },
                    Some(Err(e)) => return Err(e.clone().into()),
                    None => break,
                }
            }
        }

        // Determine whether to move on to the next limit order.
        // We do this if the limit order is fully filled.
        if limit_order.remaining().is_zero() {
            // Advance to the next limit order.
            match limit_orders.next() {
                Some(Ok(v)) => {
                    (limit_price, limit_order) = v;
                    limit_base_sold = Uint128::ZERO;
                    limit_quote_bought = Uint128::ZERO;
                },
                Some(Err(e)) => return Err(e.clone().into()),
                None => break,
            }
        }
    }

    // If a limit order is left over partially filled, return it. It will be
    // then matched against other limit orders. See `cron_execute` function.
    if limit_order.remaining().is_non_zero() {
        Ok((outcomes, Some((limit_price, limit_order))))
    } else {
        Ok((outcomes, None))
    }
}

pub fn match_market_asks_with_limit_bids<M, L>(
    market_orders: &mut M,
    limit_orders: &mut Peekable<L>,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    current_block_height: u64,
) -> anyhow::Result<(
    HashMap<ExtendedOrderId, FillingOutcome>,
    Option<(Udec128, Order)>,
)>
where
    M: Iterator<Item = (OrderId, MarketOrder)>,
    L: Iterator<Item = StdResult<(Udec128, Order)>>,
{
    todo!();
}

fn new_market_bid_filling_outcome(
    market_order: MarketOrder,
    market_base_bought: Uint128,
    market_quote_sold: Uint128,
    fee_rate: Udec128,
) -> MathResult<FillingOutcome> {
    let fee_base = market_base_bought.checked_mul_dec_ceil(fee_rate)?;
    let refund_base = market_base_bought.checked_sub(fee_base)?;

    Ok(FillingOutcome {
        order_direction: Direction::Bid,
        order: Order::Market(market_order),
        // Note: for market bids, amounts are denoted in the quote asset.
        filled: market_quote_sold,
        clearing_price: Udec128::checked_from_ratio(market_quote_sold, market_base_bought)?,
        cleared: market_order.remaining.is_zero(),
        refund_base,
        // Note: market orders are immediate-or-cancel, so refund the remaining.
        refund_quote: market_order.remaining,
        fee_base,
        fee_quote: Uint128::ZERO,
    })
}

fn new_market_ask_filling_outcome(
    market_order: MarketOrder,
    market_base_sold: Uint128,
    market_quote_bought: Uint128,
    fee_rate: Udec128,
) -> MathResult<FillingOutcome> {
    let fee_quote = market_quote_bought.checked_mul_dec_ceil(fee_rate)?;
    let refund_quote = market_quote_bought.checked_sub(fee_quote)?;

    Ok(FillingOutcome {
        order_direction: Direction::Ask,
        order: Order::Market(market_order),
        filled: market_base_sold,
        clearing_price: Udec128::checked_from_ratio(market_quote_bought, market_base_sold)?,
        cleared: market_order.remaining.is_zero(),
        refund_base: Uint128::ZERO,
        refund_quote,
        fee_base: Uint128::ZERO,
        fee_quote,
    })
}

fn new_limit_bid_filling_outcome(
    limit_order: Order,
    limit_base_bought: Uint128,
    limit_quote_sold: Uint128,
    fee_rate: Udec128,
) -> MathResult<FillingOutcome> {
    let fee_base = limit_base_bought.checked_mul_dec_ceil(fee_rate)?;
    let refund_base = limit_base_bought.checked_sub(fee_base)?;

    Ok(FillingOutcome {
        order_direction: Direction::Bid,
        order: limit_order,
        filled: limit_quote_sold,
        clearing_price: Udec128::checked_from_ratio(limit_quote_sold, limit_base_bought)?,
        cleared: limit_order.remaining().is_zero(),
        refund_base,
        // Note: limit orders are good-until-cancel, so do NOT refund the remaining.
        refund_quote: Uint128::ZERO,
        fee_base,
        fee_quote: Uint128::ZERO,
    })
}

fn new_limit_ask_filling_outcome(
    limit_order: Order,
    limit_base_sold: Uint128,
    limit_quote_bought: Uint128,
    fee_rate: Udec128,
) -> MathResult<FillingOutcome> {
    let fee_quote = limit_quote_bought.checked_mul_dec_ceil(fee_rate)?;
    let refund_quote = limit_quote_bought.checked_sub(fee_quote)?;

    Ok(FillingOutcome {
        order_direction: Direction::Ask,
        order: limit_order,
        filled: limit_base_sold,
        clearing_price: Udec128::checked_from_ratio(limit_quote_bought, limit_base_sold)?,
        cleared: limit_order.remaining().is_zero(),
        refund_base: Uint128::ZERO,
        refund_quote,
        fee_base: Uint128::ZERO,
        fee_quote,
    })
}

/// Determine the fee rate for the limit order:
/// - if it's a passive order, it's not charged any fee;
/// - if it was created at a previous block height, then it's charged the maker fee rate;
/// - otherwise, it's charged the taker fee rate.
fn limit_order_fee_rate(
    limit_order: &Order,
    current_block_height: u64,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
) -> Udec128 {
    match limit_order.created_at_block_height() {
        None => Udec128::ZERO,
        Some(block_height) if block_height < current_block_height => maker_fee_rate,
        Some(_) => taker_fee_rate,
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::PassiveOrder,
        grug::{Addr, hash_map},
        std::str::FromStr,
        test_case::test_case,
    };

    #[test_case(
        vec![],
        vec![],
        hash_map! {},
        None;
        "nothing"
    )]
    #[test_case(
        vec![
            (1, MarketOrder {
                user: Addr::mock(1),
                id: 1,
                amount: Uint128::new(200_000),
                remaining: Uint128::new(200_000),
                max_slippage: Udec128::ZERO,
            }),
        ],
        vec![
            (Udec128::from_str("200").unwrap(), Order::Passive(PassiveOrder {
                id: 2,
                price: Udec128::from_str("200").unwrap(),
                amount: Uint128::new(1000),
                remaining: Uint128::new(1000),
            })),
        ],
        hash_map! {
            ExtendedOrderId::User(1) => FillingOutcome {
                order_direction: Direction::Bid,
                order: Order::Market(MarketOrder {
                    user: Addr::mock(1),
                    id: 1,
                    amount: Uint128::new(200_000), // in quote
                    remaining: Uint128::ZERO,
                    max_slippage: Udec128::ZERO,
                }),
                filled: Uint128::new(200_000),
                clearing_price: Udec128::from_str("200").unwrap(),
                cleared: true,
                refund_base: Uint128::new(1000),
                refund_quote: Uint128::ZERO,
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
            ExtendedOrderId::Passive(2) => FillingOutcome {
                order_direction: Direction::Ask,
                order: Order::Passive(PassiveOrder {
                    id: 2,
                    price: Udec128::from_str("200").unwrap(),
                    amount: Uint128::new(1_000),
                    remaining: Uint128::ZERO,
                }),
                filled: Uint128::new(1_000),
                clearing_price: Udec128::from_str("200").unwrap(),
                cleared: true,
                refund_base: Uint128::ZERO,
                refund_quote: Uint128::new(200_000),
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
        },
        None;
        "1 limit order, 1 market order; exactly equal amounts"
    )]
    #[test_case(
        vec![
            (1, MarketOrder {
                user: Addr::mock(1),
                id: 1,
                amount: Uint128::new(200_000),
                remaining: Uint128::new(200_000),
                max_slippage: Udec128::from_str("0.005").unwrap(),
            }),
        ],
        vec![
            (Udec128::from_str("200").unwrap(), Order::Passive(PassiveOrder {
                id: 2,
                price: Udec128::from_str("200").unwrap(),
                amount: Uint128::new(500),
                remaining: Uint128::new(500),
            })),
            (Udec128::from_str("205").unwrap(), Order::Passive(PassiveOrder {
                id: 3,
                price: Udec128::from_str("205").unwrap(),
                amount: Uint128::new(500),
                remaining: Uint128::new(500),
            })),
        ],
        hash_map! {
            ExtendedOrderId::User(1) => FillingOutcome {
                order_direction: Direction::Bid,
                order: Order::Market(MarketOrder {
                    user: Addr::mock(1),
                    id: 1,
                    amount: Uint128::new(200_000), // in quote
                    remaining: Uint128::new(200_000 - 125_625),
                    max_slippage: Udec128::from_str("0.005").unwrap(),
                }),
                filled: Uint128::new(125_625),
                clearing_price: Udec128::from_str("201").unwrap(), // 200 * 1.005 = 125,625 / 625
                cleared: false,
                refund_base: Uint128::new(625),
                refund_quote: Uint128::new(200_000 - 125_625),
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
            ExtendedOrderId::Passive(2) => FillingOutcome {
                order_direction: Direction::Ask,
                order: Order::Passive(PassiveOrder {
                    id: 2,
                    price: Udec128::from_str("200").unwrap(),
                    amount: Uint128::new(500),
                    remaining: Uint128::ZERO,
                }),
                filled: Uint128::new(500),
                clearing_price: Udec128::from_str("200").unwrap(),
                cleared: true,
                refund_base: Uint128::ZERO,
                refund_quote: Uint128::new(500 * 200),
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
            ExtendedOrderId::Passive(3) => FillingOutcome {
                order_direction: Direction::Ask,
                order: Order::Passive(PassiveOrder {
                    id: 3,
                    price: Udec128::from_str("205").unwrap(),
                    amount: Uint128::new(500),
                    remaining: Uint128::new(500 - 125),
                }),
                filled: Uint128::new(125),
                clearing_price: Udec128::from_str("205").unwrap(),
                cleared: false,
                refund_base: Uint128::ZERO,
                refund_quote: Uint128::new(125 * 205),
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
        },
        Some((Udec128::from_str("205").unwrap(), Order::Passive(PassiveOrder {
            id: 3,
            price: Udec128::from_str("205").unwrap(),
            amount: Uint128::new(500),
            remaining: Uint128::new(500 - 125),
        })));
        "2 limit orders, 1 market order; the 2nd limit order has left-over"
    )]
    #[test_case(
        vec![
            (1, MarketOrder {
                user: Addr::mock(1),
                id: 1,
                amount: Uint128::new(200_000),
                remaining: Uint128::new(200_000),
                max_slippage: Udec128::from_str("0.005").unwrap(),
            }),
            (4, MarketOrder {
                user: Addr::mock(4),
                id: 4,
                amount: Uint128::new(200_000),
                remaining: Uint128::new(200_000),
                max_slippage: Udec128::from_str("0.005").unwrap(),
            }),
        ],
        vec![
            (Udec128::from_str("200").unwrap(), Order::Passive(PassiveOrder {
                id: 2,
                price: Udec128::from_str("200").unwrap(),
                amount: Uint128::new(500),
                remaining: Uint128::new(500),
            })),
            (Udec128::from_str("205").unwrap(), Order::Passive(PassiveOrder {
                id: 3,
                price: Udec128::from_str("205").unwrap(),
                amount: Uint128::new(500),
                remaining: Uint128::new(500),
            })),
        ],
        hash_map! {
            ExtendedOrderId::User(1) => FillingOutcome {
                order_direction: Direction::Bid,
                order: Order::Market(MarketOrder {
                    user: Addr::mock(1),
                    id: 1,
                    amount: Uint128::new(200_000), // in quote
                    remaining: Uint128::new(200_000 - 125_625),
                    max_slippage: Udec128::from_str("0.005").unwrap(),
                }),
                filled: Uint128::new(125_625),
                clearing_price: Udec128::from_str("201").unwrap(), // 200 * 1.005 = 125,625 / 625
                cleared: false,
                refund_base: Uint128::new(625),
                refund_quote: Uint128::new(200_000 - 125_625),
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
            ExtendedOrderId::Passive(2) => FillingOutcome {
                order_direction: Direction::Ask,
                order: Order::Passive(PassiveOrder {
                    id: 2,
                    price: Udec128::from_str("200").unwrap(),
                    amount: Uint128::new(500),
                    remaining: Uint128::ZERO,
                }),
                filled: Uint128::new(500),
                clearing_price: Udec128::from_str("200").unwrap(),
                cleared: true,
                refund_base: Uint128::ZERO,
                refund_quote: Uint128::new(500 * 200),
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
            ExtendedOrderId::Passive(3) => FillingOutcome {
                order_direction: Direction::Ask,
                order: Order::Passive(PassiveOrder {
                    id: 3,
                    price: Udec128::from_str("205").unwrap(),
                    amount: Uint128::new(500),
                    remaining: Uint128::ZERO,
                }),
                filled: Uint128::new(500),
                clearing_price: Udec128::from_str("205").unwrap(),
                cleared: true,
                refund_base: Uint128::ZERO,
                refund_quote: Uint128::new(500 * 205),
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
            ExtendedOrderId::User(4) => FillingOutcome {
                order_direction: Direction::Bid,
                order: Order::Market(MarketOrder {
                    user: Addr::mock(4),
                    id: 4,
                    amount: Uint128::new(200_000), // in quote
                    remaining: Uint128::new(200_000 - 76_875),
                    max_slippage: Udec128::from_str("0.005").unwrap(),
                }),
                filled: Uint128::new(76_875), // 375 * 205
                clearing_price: Udec128::from_str("205").unwrap(),
                cleared: false,
                refund_base: Uint128::new(375),
                refund_quote: Uint128::new(200_000 - 76_875),
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            },
        },
        None;
        "2 limit orders, 2 market orders; the 2nd market order has left-over"
    )]
    fn matching_market_bids_with_limit_asks(
        market_orders: Vec<(OrderId, MarketOrder)>,
        limit_orders: Vec<(Udec128, Order)>,
        expected_outcomes: HashMap<ExtendedOrderId, FillingOutcome>,
        expected_left_over_limit_order: Option<(Udec128, Order)>,
    ) {
        let mut market_orders = market_orders.into_iter();
        let mut limit_orders = limit_orders.into_iter().map(Ok).peekable();

        let (outcomes, left_over_limit_order) = match_market_bids_with_limit_asks(
            &mut market_orders,
            &mut limit_orders,
            Udec128::ZERO,
            Udec128::ZERO,
            0,
        )
        .unwrap();

        assert_eq!(outcomes, expected_outcomes);
        assert_eq!(left_over_limit_order, expected_left_over_limit_order);
    }
}
