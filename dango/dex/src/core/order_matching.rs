use {
    super::FillingOutcome,
    crate::{LIMIT_ORDERS, LimitOrder, MarketOrder, Order},
    dango_types::dex::{Direction, OrderId},
    grug::{MultiplyFraction, Number, NumberConst, StdError, StdResult, Udec128, Uint128},
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

pub fn match_market_bid_orders<M, L>(
    mut market_bids: M,
    mut limit_asks: Peekable<L>,
) -> StdResult<Vec<FillingOutcome>>
where
    M: Iterator<Item = StdResult<(OrderId, MarketOrder)>>,
    L: Iterator<Item = StdResult<((Udec128, OrderId), LimitOrder)>>,
{
    let mut market_bid = market_bids.next().transpose()?;
    let mut filling_outcomes = BTreeMap::<OrderId, FillingOutcome>::new();

    let best_price = if let Some(Ok(((price, _), _))) = limit_asks.peek() {
        price.clone()
    } else {
        // Return early if there are no limit orders
        return Ok(Vec::new());
    };

    loop {
        let Some(Ok(((price, ask_id), ref mut ask))) = limit_asks.peek_mut() else {
            break;
        };

        let Some((bid_id, mut bid)) = market_bid else {
            break;
        };

        let max_price = Udec128::ONE
            .checked_add(bid.max_slippage)?
            .checked_mul(best_price)?;

        // Populate the filling outcomes map with the initial values
        filling_outcomes.entry(bid_id).or_insert(FillingOutcome {
            order_direction: Direction::Bid,
            order_price: *price,
            order_id: bid_id,
            order: Order::Market(bid.clone()),
            filled: Uint128::ZERO,
            cleared: false,
            refund_base: Uint128::ZERO,
            refund_quote: Uint128::ZERO,
            fee_base: Uint128::ZERO,
            fee_quote: Uint128::ZERO,
        });
        filling_outcomes.entry(*ask_id).or_insert(FillingOutcome {
            order_direction: Direction::Ask,
            order_price: *price,
            order_id: *ask_id,
            order: Order::Limit(ask.clone()),
            filled: Uint128::ZERO,
            cleared: false,
            refund_base: Uint128::ZERO,
            refund_quote: Uint128::ZERO,
            fee_base: Uint128::ZERO,
            fee_quote: Uint128::ZERO,
        });

        let bid_amount_in_quote = if *price <= max_price {
            bid.amount
        } else {
            let FillingOutcome {
                order_price: current_avg_price,
                filled,
                ..
            } = filling_outcomes.get(&bid_id).unwrap();

            let price_ratio = current_avg_price
                .checked_sub(max_price)?
                .checked_div(max_price.checked_sub(*price)?)?;

            filled.checked_mul_dec_floor(price_ratio)?
        };

        // Convert the bid amount in quote to base asset amount using the limit order price
        let bid_amount_in_base = bid_amount_in_quote.checked_div_dec_floor(*price)?;

        // For a BUY market order the amount is in terms of the quote asset. So we must convert it to
        // base asset amount using the limit order price.
        let (filled_amount, price, ask_id, bid_id) = match bid_amount_in_base.cmp(&ask.remaining) {
            Ordering::Less => {
                // The market order is smaller than the limit order, so we can directly match it
                // take the next market order and decrement the remaining amount of the limit order
                market_bid = market_bids.next().transpose()?;
                ask.remaining.checked_sub_assign(bid_amount_in_base)?;
                (bid_amount_in_base, price.clone(), ask_id.clone(), bid_id)
            },
            Ordering::Equal => {
                // The market order is equal to the limit order, so we can directly match it
                // take the next market order and the next limit order
                market_bid = market_bids.next().transpose()?;

                // Clone values so we can next the limit order iterator
                let return_tuple = (bid_amount_in_base, price.clone(), ask_id.clone(), bid_id);

                // Pop the limit order iterator
                limit_asks.next();

                return_tuple
            },
            Ordering::Greater => {
                // The market order is greater than the limit order, so we can directly match it
                // take the next limit order and decrement the market order amount
                bid.amount
                    .checked_sub_assign(ask.remaining.checked_mul_dec_ceil(*price)?)?;

                // Clone values so we can next the limit order iterator
                let return_tuple = (ask.amount.clone(), price.clone(), ask_id.clone(), bid_id);

                // Pop the limit order iterator
                limit_asks.next();

                return_tuple
            },
        };

        let bid_filling_outcome = filling_outcomes.get_mut(&bid_id).unwrap();
        bid_filling_outcome
            .filled
            .checked_add_assign(filled_amount)?;
        // TODO handle fees

        let ask_filling_outcome = filling_outcomes.get_mut(&ask_id).unwrap();
        ask_filling_outcome
            .filled
            .checked_add_assign(filled_amount)?;

        // Update average price of the market order
        ask_filling_outcome.order_price = Udec128::checked_from_ratio(
            ask_filling_outcome
                .filled
                .checked_mul_dec(ask_filling_outcome.order_price)?
                .checked_add(filled_amount.checked_mul_dec(price)?)?,
            ask_filling_outcome.filled.checked_add(filled_amount)?,
        )?;
        // TODO handle fees
    }

    Ok(filling_outcomes.into_values().collect())
}

pub fn match_market_ask_orders<M, L>(
    mut market_asks: M,
    mut limit_bids: Peekable<L>,
) -> StdResult<Vec<FillingOutcome>>
where
    M: Iterator<Item = StdResult<(OrderId, MarketOrder)>>,
    L: Iterator<Item = StdResult<((Udec128, OrderId), LimitOrder)>>,
{
    let mut market_ask = market_asks.next().transpose()?;
    let mut filling_outcomes = BTreeMap::<OrderId, FillingOutcome>::new();

    let best_price = if let Some(Ok(((price, _), _))) = limit_bids.peek() {
        price.clone()
    } else {
        // Return early if there are no limit orders
        return Ok(Vec::new());
    };

    loop {
        let Some(Ok(((price, bid_id), ref mut bid))) = limit_bids.peek_mut() else {
            break;
        };

        let Some((ask_id, mut ask)) = market_ask else {
            break;
        };

        let min_price = Udec128::ONE
            .checked_sub(ask.max_slippage)?
            .checked_mul(best_price)?;

        // For a market ASK order the amount is in terms of the base asset. So we can directly
        // match it against the limit order remaining amount

        // Populate the filling outcomes map with the initial values
        filling_outcomes.entry(*bid_id).or_insert(FillingOutcome {
            order_direction: Direction::Bid,
            order_price: *price,
            order_id: *bid_id,
            order: Order::Limit(bid.clone()),
            filled: Uint128::ZERO,
            cleared: false,
            refund_base: Uint128::ZERO,
            refund_quote: Uint128::ZERO,
            fee_base: Uint128::ZERO,
            fee_quote: Uint128::ZERO,
        });
        filling_outcomes.entry(ask_id).or_insert(FillingOutcome {
            order_direction: Direction::Ask,
            order_price: *price,
            order_id: ask_id,
            order: Order::Market(ask.clone()),
            filled: Uint128::ZERO,
            cleared: false,
            refund_base: Uint128::ZERO,
            refund_quote: Uint128::ZERO,
            fee_base: Uint128::ZERO,
            fee_quote: Uint128::ZERO,
        });

        let ask_amount = if *price >= min_price {
            ask.amount
        } else {
            let FillingOutcome {
                order_price: current_avg_price,
                filled,
                ..
            } = filling_outcomes.get(&ask_id).unwrap();

            let price_ratio = current_avg_price
                .checked_sub(min_price)?
                .checked_div(min_price.checked_sub(*price)?)?;

            filled.checked_mul_dec_floor(price_ratio)?
        };

        let (filled_amount, price, ask_id, bid_id) = match ask_amount.cmp(&bid.remaining) {
            Ordering::Less => {
                // The market ask order is smaller than the limit order, so we can directly match it
                // take the next market order and decrement the remaining amount of the limit order
                market_ask = market_asks.next().transpose()?;
                bid.remaining.checked_sub_assign(ask_amount)?;
                (ask_amount, price.clone(), ask_id, bid_id.clone())
            },
            Ordering::Equal => {
                // The market order is equal to the limit order, so we can directly match it
                // take the next market order and the next limit order
                market_ask = market_asks.next().transpose()?;

                // Clone values so we can next the limit order iterator
                let return_tuple = (ask_amount, price.clone(), ask_id, bid_id.clone());

                // Pop the limit order iterator
                limit_bids.next();

                return_tuple
            },
            Ordering::Greater => {
                // The market order is greater than the limit order, so we can directly match it
                // take the next limit order and decrement the market order amount
                ask.amount.checked_sub_assign(bid.remaining)?;

                // Clone values so we can next the limit order iterator
                let return_tuple = (bid.amount.clone(), price.clone(), ask_id, bid_id.clone());

                // Pop the limits iterator
                limit_bids.next();

                return_tuple
            },
        };

        let bid_filling_outcome = filling_outcomes.get_mut(&bid_id).unwrap();
        bid_filling_outcome
            .filled
            .checked_add_assign(filled_amount)?;
        // TODO handle fees

        let ask_filling_outcome = filling_outcomes.get_mut(&ask_id).unwrap();
        ask_filling_outcome
            .filled
            .checked_add_assign(filled_amount)?;

        // Update average price of the market order
        ask_filling_outcome.order_price = Udec128::checked_from_ratio(
            ask_filling_outcome
                .filled
                .checked_mul_dec(ask_filling_outcome.order_price)?
                .checked_add(filled_amount.checked_mul_dec(price)?)?,
            ask_filling_outcome.filled.checked_add(filled_amount)?,
        )?;
        // TODO handle fees
    }

    Ok(filling_outcomes.into_values().collect())
}
