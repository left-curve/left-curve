use {
    super::FillingOutcome,
    crate::{LimitOrder, MarketOrder, Order},
    dango_types::dex::{Direction, OrderId},
    grug::{IsZero, MultiplyFraction, Number, NumberConst, StdResult, Udec128, Uint128},
    std::{cmp::Ordering, collections::BTreeMap, iter::Peekable},
};

pub struct MatchingOutcome {
    /// The range of prices that achieve the biggest trading volume.
    /// `None` if no match is found.
    ///
    /// All prices in this range achieve the same volume. It's up to the caller
    /// to decide which price to use: the lowest, the highest, or the midpoint.
    pub range: Option<(Udec128, Udec128)>,
    /// The amount of trading volume, measured as the amount of the base asset.
    pub volume: Uint128,
    /// The BUY orders that have found a match.
    pub bids: Vec<((Udec128, OrderId), LimitOrder)>,
    /// The SELL orders that have found a match.
    pub asks: Vec<((Udec128, OrderId), LimitOrder)>,
}

/// Given the standing BUY and SELL orders in the book, find range of prices
/// that maximizes the trading volume.
///
/// ## Inputs:
///
/// - `bid_iter`: An iterator over the BUY orders in the book. This should
///   follow the **price-time priority**, meaning it should return the order
///   with the best price (in the case of BUY orders, the highest price) first;
///   for orders the same price, the oldest one first.
/// - `ask_iter`: An iterator over the SELL orders in the book that similarly
///   follows the price-time priority.
pub fn match_limit_orders<B, A>(mut bid_iter: B, mut ask_iter: A) -> StdResult<MatchingOutcome>
where
    B: Iterator<Item = StdResult<((Udec128, OrderId), LimitOrder)>>,
    A: Iterator<Item = StdResult<((Udec128, OrderId), LimitOrder)>>,
{
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

    Ok(MatchingOutcome {
        range,
        volume: bid_volume.min(ask_volume),
        bids,
        asks,
    })
}

pub fn match_market_orders<M, L>(
    market_orders: &mut M,
    limit_orders: &mut Peekable<L>,
    market_order_direction: Direction,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    current_block_height: u64,
) -> anyhow::Result<Vec<FillingOutcome>>
where
    M: Iterator<Item = StdResult<(OrderId, MarketOrder)>>,
    L: Iterator<Item = StdResult<((Udec128, OrderId), LimitOrder)>>,
{
    let mut maybe_market_order = market_orders.next().transpose()?;
    let mut filling_outcomes = BTreeMap::<OrderId, FillingOutcome>::new();

    // Limit order direction is assumed to be opposite to the market order direction
    let limit_order_direction = match market_order_direction {
        Direction::Bid => Direction::Ask,
        Direction::Ask => Direction::Bid,
    };

    // The best possible price is the price of the first limit order in the book
    let best_price = match limit_orders.peek_mut() {
        Some(Ok(((price, _), _))) => price.clone(),
        Some(Err(e)) => return Err(e.clone().into()),
        None => return Ok(Vec::new()), // Return early if there are no limit orders
    };

    // Iterate over the limit orders and market orders until one of them is exhausted.
    // Since a market order can partially fill a limit order, and that limit order should
    // remain in the book partially filled, we mutably peek the limit orders iterator and
    // only advance it when the market order amount is greater than or equal to the remaining
    // amount of the limit order.
    //
    // This is not the case for the market orders. They are matched in the order they were
    // received, and do not remain after matching is completed.
    loop {
        let (price, limit_order_id, limit_order) = match limit_orders.peek_mut() {
            Some(Ok(((price, limit_order_id), ref mut limit_order))) => {
                (price, limit_order_id, limit_order)
            },
            Some(Err(e)) => return Err(e.clone().into()),
            None => break,
        };

        let Some((market_order_id, mut market_order)) = maybe_market_order else {
            break;
        };

        // Calculate the cutoff price for the current market order
        let cutoff_price = match market_order_direction {
            Direction::Bid => Udec128::ONE
                .checked_add(market_order.max_slippage)?
                .checked_mul(best_price)?,
            Direction::Ask => Udec128::ONE
                .checked_sub(market_order.max_slippage)?
                .checked_mul(best_price)?,
        };

        // The direction of the comparison depends on whether the market order
        // is a BUY or a SELL.
        let price_is_worse_than_cutoff = match market_order_direction {
            Direction::Bid => *price > cutoff_price,
            Direction::Ask => *price < cutoff_price,
        };

        // If the price is not worse than the cutoff price, we can match the market order
        // against the limit order in full. Otherwise, we need to calculate the amount of the
        // market order that can be matched against the limit order, before the average
        // execution price of the order becomes worse than the cutoff price. We get
        // the amount by solving the equation:
        //
        // (avg_price * filled + amount * price) / (filled + amount) = cutoff_price
        //
        // We solve for `amount` to get:
        //
        // amount = filled * (avg_price - cutoff_price) / (cutoff_price - price)
        //
        // We round down the result to ensure that the average price of the market order
        // does not exceed the cutoff price.
        let market_order_amount = if !price_is_worse_than_cutoff {
            market_order.amount
        } else {
            let FillingOutcome {
                order_price: current_avg_price,
                filled,
                ..
            } = filling_outcomes.get(&market_order_id).unwrap();

            let price_ratio = current_avg_price
                .checked_sub(cutoff_price)?
                .checked_div(cutoff_price.checked_sub(*price)?)?;

            filled.checked_mul_dec_floor(price_ratio)?
        };

        // Convert the market order amount to the base asset for BUY orders
        let market_order_amount_in_base = match market_order_direction {
            Direction::Bid => market_order_amount.checked_div_dec_floor(*price)?,
            Direction::Ask => market_order_amount,
        };

        // For a market ASK order the amount is in terms of the base asset. So we can directly
        // match it against the limit order remaining amount
        let (filled_amount, price, limit_order_id, market_order_id, limit_order, market_order) =
            match market_order_amount_in_base.cmp(&limit_order.remaining) {
                // The market ask order is smaller than the limit order so we advance the market
                // orders iterator and decrement the limit order remaining amount
                Ordering::Less => {
                    maybe_market_order = market_orders.next().transpose()?;
                    limit_order
                        .remaining
                        .checked_sub_assign(market_order_amount_in_base)?;
                    (
                        market_order_amount_in_base,
                        price.clone(),
                        limit_order_id.clone(),
                        market_order_id,
                        limit_order.clone(),
                        market_order.clone(),
                    )
                },
                // The market order amount is equal to the limit order remaining amount, so we can
                // match both in full, and advance both iterators.
                Ordering::Equal => {
                    println!("orders are equal");
                    maybe_market_order = market_orders.next().transpose()?;
                    limit_order.remaining = Uint128::ZERO;

                    // Clone values so we can next the limit order iterator
                    let return_tuple = (
                        market_order_amount_in_base,
                        price.clone(),
                        limit_order_id.clone(),
                        market_order_id,
                        limit_order.clone(),
                        market_order.clone(),
                    );

                    println!("limit order remaining: {:?}", limit_order);

                    // Pop the limit order iterator
                    limit_orders.next();

                    return_tuple
                },
                // The market order amount is greater than the limit order remaining amount,
                // so we advance fully match the limit, decrement the market order amount and
                // advance the limit orders iterator
                Ordering::Greater => {
                    let limit_remaining_amount = limit_order.remaining;
                    market_order
                        .amount
                        .checked_sub_assign(limit_remaining_amount)?;
                    limit_order.remaining = Uint128::ZERO;

                    // Clone values so we can next the limit order iterator
                    let return_tuple = (
                        limit_remaining_amount,
                        price.clone(),
                        limit_order_id.clone(),
                        market_order_id,
                        limit_order.clone(),
                        market_order.clone(),
                    );

                    // Pop the limits iterator
                    limit_orders.next();

                    return_tuple
                },
            };

        // Update the filling outcomes
        let limit_order_fee_rate = if limit_order.created_at_block_height < current_block_height {
            maker_fee_rate
        } else {
            taker_fee_rate
        };
        _update_filling_outcome(
            &mut filling_outcomes,
            Order::Limit(limit_order),
            limit_order_id,
            limit_order_direction,
            filled_amount,
            price,
            limit_order_fee_rate,
        )?;
        _update_filling_outcome(
            &mut filling_outcomes,
            Order::Market(market_order),
            market_order_id,
            market_order_direction,
            filled_amount,
            price,
            taker_fee_rate,
        )?;
    }

    Ok(filling_outcomes.into_values().collect())
}

fn _update_filling_outcome(
    filling_outcomes: &mut BTreeMap<OrderId, FillingOutcome>,
    order: Order,
    order_id: OrderId,
    order_direction: Direction,
    filled_amount: Uint128,
    price: Udec128,
    fee_rate: Udec128,
) -> StdResult<()> {
    let filling_outcome = filling_outcomes.entry(order_id).or_insert(FillingOutcome {
        order_direction,
        order_price: price,
        order_id,
        order: order.clone(),
        filled: Uint128::ZERO,
        cleared: false,
        refund_base: match order {
            Order::Limit(_) => Uint128::ZERO,
            Order::Market(market_order) => match order_direction {
                Direction::Bid => Uint128::ZERO,
                Direction::Ask => market_order.amount,
            },
        },
        refund_quote: match order {
            Order::Limit(_) => Uint128::ZERO,
            Order::Market(market_order) => match order_direction {
                Direction::Bid => market_order.amount.checked_div_dec_floor(price)?,
                Direction::Ask => Uint128::ZERO,
            },
        },
        fee_base: Uint128::ZERO,
        fee_quote: Uint128::ZERO,
    });
    match order {
        Order::Limit(limit_order) => {
            filling_outcome.cleared = limit_order.remaining.is_zero();
        },
        Order::Market(_) => {
            filling_outcome.order_price = Udec128::checked_from_ratio(
                filling_outcome
                    .filled
                    .checked_mul_dec(filling_outcome.order_price)?
                    .checked_add(filled_amount.checked_mul_dec(price)?)?,
                filling_outcome.filled.checked_add(filled_amount)?,
            )?;
            match order_direction {
                Direction::Bid => {
                    println!("in bid arm");
                    println!("refund quote: {:?}", filling_outcome.refund_quote);
                    println!(
                        "filled amount * price: {:?}",
                        filled_amount.checked_mul_dec_ceil(price)?
                    );

                    filling_outcome
                        .refund_quote
                        .checked_sub_assign(filled_amount.checked_mul_dec_ceil(price)?)?;
                },
                Direction::Ask => {
                    println!("in ask arm");
                    println!("refund base: {:?}", filling_outcome.refund_base);
                    filling_outcome
                        .refund_base
                        .checked_sub_assign(filled_amount)?;
                },
            }
        },
    }

    filling_outcome.filled.checked_add_assign(filled_amount)?;
    filling_outcome.order = order;

    match order_direction {
        Direction::Bid => {
            let fee_amount = filled_amount.checked_mul_dec_ceil(fee_rate)?;

            filling_outcome.fee_base.checked_add_assign(fee_amount)?;
            filling_outcome
                .refund_base
                .checked_add_assign(filled_amount.checked_sub(fee_amount)?)?;
        },
        Direction::Ask => {
            let filled_amount_in_quote = filled_amount.checked_mul_dec_floor(price)?;
            let fee_amount_in_quote = filled_amount_in_quote.checked_mul_dec_ceil(fee_rate)?;

            filling_outcome
                .fee_quote
                .checked_add_assign(fee_amount_in_quote)?;
            filling_outcome
                .refund_quote
                .checked_add_assign(filled_amount_in_quote.checked_sub(fee_amount_in_quote)?)?;
        },
    }

    Ok(())
}
