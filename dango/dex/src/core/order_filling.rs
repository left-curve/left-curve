use {
    crate::Order,
    dango_types::dex::{Direction, OrderId},
    grug::{IsZero, MultiplyFraction, NumberConst, StdResult, Udec128, Uint128},
};

#[derive(Debug)]
pub struct FillingOutcome {
    pub order_direction: Direction,
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
    outcome.extend(fill_asks(asks, clearing_price, volume)?);
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
            order_direction: Direction::Bid,
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
    clearing_price: Udec128,
    mut volume: Uint128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(asks.len());

    for ((order_price, order_id), mut order) in asks {
        let filled = order.remaining.min(volume);

        order.remaining -= filled;
        volume -= filled;

        outcome.push(FillingOutcome {
            order_direction: Direction::Ask,
            order_price,
            order_id,
            order,
            filled,
            cleared: order.remaining.is_zero(),
            refund_base: Uint128::ZERO,
            refund_quote: filled.checked_mul_dec_floor(clearing_price)?,
        });

        if volume.is_zero() {
            break;
        }
    }

    Ok(outcome)
}
