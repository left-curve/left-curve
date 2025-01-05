use {
    crate::Order,
    dango_types::orderbook::OrderId,
    grug::{IsZero, MultiplyFraction, Number, NumberConst, StdResult, Udec128, Uint128},
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
    pub bids: Vec<((Udec128, OrderId), Order)>,
    /// The SELL orders that have found a match.
    pub asks: Vec<((Udec128, OrderId), Order)>,
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
    B: Iterator<Item = StdResult<((Udec128, OrderId), Order)>>,
    A: Iterator<Item = StdResult<((Udec128, OrderId), Order)>>,
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

pub struct FillingOutcome {
    pub order_price: Udec128,
    pub order_id: OrderId,
    /// The order with the `filled` amount updated.
    pub order: Order,
    /// The amount, measured in the base asset, that has been filled.
    pub filled: Uint128,
    /// Whether the order has been fully filled.
    pub cleared: bool,
    /// Amount of base asset that should be refunded to the trader.
    pub refund_base: Uint128,
    /// Amount of quote asset that should be refunded to the trader.
    pub refund_quote: Uint128,
}

/// Clear the orders given a clearing price and volume.
pub fn fill_orders(
    bids: Vec<((Udec128, OrderId), Order)>,
    asks: Vec<((Udec128, OrderId), Order)>,
    clearing_price: Udec128,
    volume: Uint128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(bids.len() + asks.len());
    outcome.extend(fill_bids(bids, clearing_price, volume)?);
    outcome.extend(fill_asks(asks, volume)?);
    Ok(outcome)
}

/// Fill the BUY orders given a clearing price and volume.
fn fill_bids(
    bids: Vec<((Udec128, OrderId), Order)>,
    clearing_price: Udec128,
    mut volume: Uint128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(bids.len());

    for ((order_price, order_id), mut order) in bids {
        let filled = order.remaining.min(volume);

        order.remaining -= filled;
        volume -= filled;

        outcome.push(FillingOutcome {
            order_price,
            order_id,
            order,
            filled,
            cleared: order.remaining.is_zero(),
            refund_base: filled,
            // If the order is filled at a price better than the limit price,
            // we need to refund the trader the unused quote asset.
            refund_quote: filled.checked_mul_dec_floor(order_price - clearing_price)?,
        });

        if volume.is_zero() {
            break;
        }
    }

    Ok(outcome)
}

/// Fill the SELL orders given a clearing price and volume.
fn fill_asks(
    asks: Vec<((Udec128, OrderId), Order)>,
    mut volume: Uint128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(asks.len());

    for ((order_price, order_id), mut order) in asks {
        let filled = order.remaining.min(volume);

        order.remaining -= filled;
        volume -= filled;

        outcome.push(FillingOutcome {
            order_price,
            order_id,
            order,
            filled,
            cleared: order.remaining.is_zero(),
            refund_base: Uint128::ZERO,
            refund_quote: filled,
        });

        if volume.is_zero() {
            break;
        }
    }

    Ok(outcome)
}
