use {
    super::FillingOutcome,
    crate::{LimitOrder, MarketOrder, Order},
    dango_types::dex::{Direction, OrderId},
    grug::{
        Inner, IsZero, MultiplyFraction, Number, NumberConst, Signed, StdResult, Udec128, Uint128,
        Unsigned,
    },
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
    market_orders: &mut Peekable<M>,
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
    if market_orders.peek().is_none() || limit_orders.peek().is_none() {
        return Ok(Vec::new());
    }

    let mut filling_outcomes = BTreeMap::<OrderId, FillingOutcome>::new();

    // Limit order direction is assumed to be opposite to the market order direction
    let limit_order_direction = match market_order_direction {
        Direction::Bid => Direction::Ask,
        Direction::Ask => Direction::Bid,
    };

    // The best possible price is the price of the first limit order in the book
    let best_price = match limit_orders.peek_mut() {
        Some(Ok(((price, _), _))) => *price,
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

        let (market_order_id, market_order) = match market_orders.peek_mut() {
            Some(Ok((market_order_id, ref mut market_order))) => (market_order_id, market_order),
            Some(Err(e)) => return Err(e.clone().into()),
            None => break,
        };

        // Calculate the cutoff price for the current market order
        let cutoff_price = match market_order_direction {
            Direction::Bid => Udec128::ONE
                .checked_add(market_order.max_slippage.into_inner())?
                .checked_mul(best_price)?,
            Direction::Ask => Udec128::ONE
                .checked_sub(market_order.max_slippage.into_inner())?
                .checked_mul(best_price)?,
        };

        // The direction of the comparison depends on whether the market order
        // is a BUY or a SELL.
        let price_is_worse_than_cutoff = match market_order_direction {
            Direction::Bid => *price > cutoff_price,
            Direction::Ask => *price < cutoff_price,
        };

        let market_order_amount_in_base = match market_order_direction {
            Direction::Bid => market_order.amount.checked_div_dec_floor(*price)?,
            Direction::Ask => market_order.amount,
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
        let market_order_amount_to_match_in_base = if !price_is_worse_than_cutoff {
            market_order_amount_in_base
        } else {
            let filling_outcome = filling_outcomes.get_mut(market_order_id).unwrap();
            let current_avg_price = filling_outcome.order_price;
            let filled = filling_outcome.filled;
            let price_ratio = current_avg_price
                .checked_into_signed()?
                .checked_sub(cutoff_price.checked_into_signed()?)?
                .checked_div(
                    cutoff_price
                        .checked_into_signed()?
                        .checked_sub(price.checked_into_signed()?)?,
                )?;

            // Calculate how much of the market order can be filled without the average
            // price of the market order exceeding the cutoff price.
            let market_order_amount_to_match_in_base = filled
                .checked_mul_dec_floor(price_ratio.checked_into_unsigned()?)?
                .min(market_order_amount_in_base);

            // Since the order is only partially filled we update the filling outcome
            // to refund the amount that was not filled.
            match market_order_direction {
                Direction::Bid => {
                    filling_outcome.refund_quote.checked_add_assign(
                        market_order.amount.checked_sub(
                            market_order_amount_to_match_in_base.checked_mul_dec_ceil(*price)?,
                        )?,
                    )?;
                },
                Direction::Ask => {
                    filling_outcome.refund_base.checked_add_assign(
                        market_order
                            .amount
                            .checked_sub(market_order_amount_to_match_in_base)?,
                    )?;
                },
            }

            market_order_amount_to_match_in_base
        };

        // For a market ASK order the amount is in terms of the base asset. So we can directly
        // match it against the limit order remaining amount
        let (filled_amount, price, limit_order_id, market_order_id, limit_order, market_order) =
            match market_order_amount_to_match_in_base.cmp(&limit_order.remaining) {
                // The market ask order is smaller than the limit order so we advance the market
                // orders iterator and decrement the limit order remaining amount
                Ordering::Less => {
                    limit_order
                        .remaining
                        .checked_sub_assign(market_order_amount_to_match_in_base)?;
                    market_order.amount = Uint128::ZERO;

                    // Clone values so we can next the market order iterator
                    let return_tuple = (
                        market_order_amount_to_match_in_base,
                        *price,
                        *limit_order_id,
                        *market_order_id,
                        *limit_order,
                        *market_order,
                    );

                    // Advance the market orders iterator
                    market_orders.next();

                    return_tuple
                },
                // The market order amount is equal to the limit order remaining amount, so we can
                // match both in full, and advance both iterators.
                Ordering::Equal => {
                    limit_order.remaining = Uint128::ZERO;
                    market_order.amount = Uint128::ZERO;

                    // Clone values so we can next the limit order iterator
                    let return_tuple = (
                        market_order_amount_to_match_in_base,
                        *price,
                        *limit_order_id,
                        *market_order_id,
                        *limit_order,
                        *market_order,
                    );

                    // Advance the both order iterators
                    limit_orders.next();
                    market_orders.next();
                    return_tuple
                },
                // The market order amount is greater than the limit order remaining amount,
                // so we advance fully match the limit, decrement the market order amount and
                // advance the limit orders iterator
                Ordering::Greater => {
                    let limit_remaining_amount = limit_order.remaining;

                    // Decrement the market order amount by the limit order remaining amount.
                    // This is done differently for BUY and SELL market orders because the amount
                    // is in terms of the quote asset for BUY orders and in terms of the base asset
                    // for SELL orders.
                    // If this is the last market order to be matched, i.e. the limit order iterator
                    // is exhausted, the market order will remain in the market orders iterator and
                    // the amount left in the market order will be refunded in `cron_execute`.
                    match market_order_direction {
                        Direction::Bid => {
                            market_order.amount.checked_sub_assign(
                                limit_remaining_amount.checked_mul_dec_ceil(*price)?,
                            )?;
                        },
                        Direction::Ask => {
                            market_order
                                .amount
                                .checked_sub_assign(limit_remaining_amount)?;
                        },
                    }

                    limit_order.remaining = Uint128::ZERO;

                    // Clone values so we can next the limit order iterator
                    let return_tuple = (
                        limit_remaining_amount,
                        *price,
                        *limit_order_id,
                        *market_order_id,
                        *limit_order,
                        *market_order,
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
        refund_base: Uint128::ZERO,
        refund_quote: Uint128::ZERO,
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
