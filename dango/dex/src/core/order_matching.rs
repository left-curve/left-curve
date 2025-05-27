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
    M: Iterator<Item = StdResult<MarketOrder>>,
    L: Iterator<Item = StdResult<((Udec128, OrderId), LimitOrder)>>,
{
    let mut market_bid = market_bids.next().transpose()?;
    let mut filling_outcomes = Vec::new();

    loop {
        let Some(Ok(((price, _), ref mut ask))) = limit_asks.peek_mut() else {
            break;
        };

        let Some(mut bid) = market_bid else {
            break;
        };

        // For a BUY market order the amount is in terms of the quote asset. So we must convert it to
        // base asset amount using the limit order price.
        let bid_amount_in_base = bid.amount.checked_div_dec_floor(*price)?;
        match bid_amount_in_base.cmp(&ask.remaining) {
            Ordering::Less => {
                // The market order is smaller than the limit order, so we can directly match it
                // take the next market order and decrement the remaining amount of the limit order
                market_bid = market_bids.next().transpose()?;
                ask.remaining.checked_sub_assign(bid_amount_in_base)?;
            },
            Ordering::Equal => {
                // The market order is equal to the limit order, so we can directly match it
                // take the next market order and the next limit order
                market_bid = market_bids.next().transpose()?;
                limit_asks.next();
            },
            Ordering::Greater => {
                // The market order is greater than the limit order, so we can directly match it
                // take the next limit order and decrement the market order amount
                bid.amount
                    .checked_sub_assign(ask.remaining.checked_mul_dec_ceil(*price)?)?;
                limit_asks.next();
            },
        }
    }

    Ok(filling_outcomes)
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

    loop {
        let Some(Ok(((price, bid_id), ref mut bid))) = limit_bids.peek_mut() else {
            break;
        };

        let Some((ask_id, mut ask)) = market_ask else {
            break;
        };

        // For a market ASK order the amount is in terms of the base asset. So we can directly
        // match it against the limit order remaining amount
        match ask.amount.cmp(&bid.remaining) {
            Ordering::Less => {
                let mut bid_outcome = filling_outcomes.entry(*bid_id).or_insert(FillingOutcome {
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
                bid_outcome.filled.checked_add_assign(ask.amount)?;
                bid_outcome.refund_base.checked_add_assign(ask.amount)?;
                // TODO: calculate fee

                let ask_outcome = filling_outcomes.entry(ask_id).or_insert(FillingOutcome {
                    order_direction: Direction::Ask,
                    order_price: *price,
                    order_id: ask_id,
                    order: Order::Market(ask),
                    filled: Uint128::ZERO,
                    cleared: false,
                    refund_base: Uint128::ZERO,
                    refund_quote: Uint128::ZERO,
                    fee_base: Uint128::ZERO,
                    fee_quote: Uint128::ZERO,
                });
                ask_outcome.filled.checked_add_assign(ask.amount)?;
                ask_outcome
                    .refund_quote
                    .checked_add_assign(ask.amount.checked_mul_dec_floor(*price)?)?;
                // TODO: calculate fee

                // The market ask order is smaller than the limit order, so we can directly match it
                // take the next market order and decrement the remaining amount of the limit order
                market_ask = market_asks.next().transpose()?;
                bid.remaining.checked_sub_assign(ask.amount)?;
            },
            Ordering::Equal => {
                // The market order is equal to the limit order, so we can directly match it
                // take the next market order and the next limit order
                market_ask = market_asks.next().transpose()?;
                limit_bids.next();
            },
            Ordering::Greater => {
                // The market order is greater than the limit order, so we can directly match it
                // take the next limit order and decrement the market order amount
                ask.amount.checked_sub_assign(bid.remaining)?;
                limit_bids.next();
            },
        }
    }

    Ok(filling_outcomes.into_values().collect())
}
