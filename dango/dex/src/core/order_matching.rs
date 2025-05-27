use {
    super::FillingOutcome,
    crate::{LimitOrder, MarketOrder, Order},
    dango_types::dex::{Direction, OrderId},
    grug::{MultiplyFraction, Number, NumberConst, StdResult, Udec128, Uint128},
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
pub fn match_orders<B, A>(mut bid_iter: B, mut ask_iter: A) -> StdResult<MatchingOutcome>
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
) -> StdResult<Vec<FillingOutcome>>
where
    M: Iterator<Item = StdResult<(OrderId, MarketOrder)>>,
    L: Iterator<Item = StdResult<((Udec128, OrderId), LimitOrder)>>,
{
    let mut maybe_market_order = market_orders.next().transpose()?;
    let mut filling_outcomes = BTreeMap::<OrderId, FillingOutcome>::new();

    let limit_order_direction = match market_order_direction {
        Direction::Bid => Direction::Ask,
        Direction::Ask => Direction::Bid,
    };

    let best_price = if let Some(Ok(((price, _), _))) = limit_orders.peek() {
        price.clone()
    } else {
        // Return early if there are no limit orders
        return Ok(Vec::new());
    };

    loop {
        let Some(Ok(((price, limit_order_id), ref mut limit_order))) = limit_orders.peek_mut()
        else {
            break;
        };

        let Some((market_order_id, mut market_order)) = maybe_market_order else {
            break;
        };

        let cutoff_price = match market_order_direction {
            Direction::Bid => Udec128::ONE
                .checked_add(market_order.max_slippage)?
                .checked_mul(best_price)?,
            Direction::Ask => Udec128::ONE
                .checked_sub(market_order.max_slippage)?
                .checked_mul(best_price)?,
        };

        // Populate the filling outcomes map with the initial values
        filling_outcomes
            .entry(*limit_order_id)
            .or_insert(FillingOutcome {
                order_direction: limit_order_direction,
                order_price: *price,
                order_id: *limit_order_id,
                order: Order::Limit(limit_order.clone()),
                filled: Uint128::ZERO,
                cleared: false,
                refund_base: Uint128::ZERO,
                refund_quote: Uint128::ZERO,
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            });
        filling_outcomes
            .entry(market_order_id)
            .or_insert(FillingOutcome {
                order_direction: market_order_direction,
                order_price: *price,
                order_id: market_order_id,
                order: Order::Market(market_order.clone()),
                filled: Uint128::ZERO,
                cleared: false,
                refund_base: Uint128::ZERO,
                refund_quote: Uint128::ZERO,
                fee_base: Uint128::ZERO,
                fee_quote: Uint128::ZERO,
            });

        let cmp = match market_order_direction {
            Direction::Bid => (*price).cmp(&cutoff_price),
            Direction::Ask => cutoff_price.cmp(price),
        };

        let market_order_amount = if cmp == Ordering::Less {
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

        let market_order_amount_in_base = match market_order_direction {
            Direction::Bid => market_order_amount.checked_div_dec_floor(*price)?,
            Direction::Ask => market_order_amount,
        };

        // For a market ASK order the amount is in terms of the base asset. So we can directly
        // match it against the limit order remaining amount
        let (filled_amount, price, limit_order_id, market_order_id, limit_cleared) =
            match market_order_amount.cmp(&limit_order.remaining) {
                Ordering::Less => {
                    // The market ask order is smaller than the limit order, so we can directly match it
                    // take the next market order and decrement the remaining amount of the limit order
                    maybe_market_order = market_orders.next().transpose()?;
                    limit_order
                        .remaining
                        .checked_sub_assign(market_order_amount_in_base)?;
                    (
                        market_order_amount_in_base,
                        price.clone(),
                        limit_order_id.clone(),
                        market_order_id,
                        false,
                    )
                },
                Ordering::Equal => {
                    // The market order is equal to the limit order, so we can directly match it
                    // take the next market order and the next limit order
                    maybe_market_order = market_orders.next().transpose()?;

                    // Clone values so we can next the limit order iterator
                    let return_tuple = (
                        market_order_amount_in_base,
                        price.clone(),
                        limit_order_id.clone(),
                        market_order_id,
                        true,
                    );

                    // Pop the limit order iterator
                    limit_orders.next();

                    return_tuple
                },
                Ordering::Greater => {
                    // The market order is greater than the limit order, so we can directly match it
                    // take the next limit order and decrement the market order amount
                    market_order
                        .amount
                        .checked_sub_assign(limit_order.remaining)?;
                    limit_order.remaining = Uint128::ZERO;

                    // Clone values so we can next the limit order iterator
                    let return_tuple = (
                        market_order_amount_in_base,
                        price.clone(),
                        limit_order_id.clone(),
                        market_order_id,
                        true,
                    );

                    // Pop the limits iterator
                    limit_orders.next();

                    return_tuple
                },
            };

        let limit_order_filling_outcome = filling_outcomes.get_mut(&limit_order_id).unwrap();
        limit_order_filling_outcome
            .filled
            .checked_add_assign(filled_amount)?;
        limit_order_filling_outcome.cleared = limit_cleared;
        // TODO handle fees

        let market_order_filling_outcome = filling_outcomes.get_mut(&market_order_id).unwrap();
        market_order_filling_outcome
            .filled
            .checked_add_assign(filled_amount)?;

        // Update average price of the market order
        market_order_filling_outcome.order_price = Udec128::checked_from_ratio(
            market_order_filling_outcome
                .filled
                .checked_mul_dec(market_order_filling_outcome.order_price)?
                .checked_add(filled_amount.checked_mul_dec(price)?)?,
            market_order_filling_outcome
                .filled
                .checked_add(filled_amount)?,
        )?;
        // TODO handle fees
    }

    Ok(filling_outcomes.into_values().collect())
}
