use {
    crate::LimitOrder,
    dango_types::dex::{Direction, OrderId},
    grug::{IsZero, MultiplyFraction, Number, NumberConst, StdResult, Udec128, Uint128},
};

#[derive(Debug)]
pub struct FillingOutcome {
    pub order_direction: Direction,
    pub order_price: Udec128,
    pub order_id: OrderId,
    /// The order with the `filled` amount updated.
    pub order: LimitOrder,
    /// The amount, measured in the base asset, that has been filled.
    pub filled: Uint128,
    /// Whether the order has been fully filled.
    pub cleared: bool,
    /// Amount of base asset that should be refunded to the trader.
    pub refund_base: Uint128,
    /// Amount of quote asset that should be refunded to the trader.
    pub refund_quote: Uint128,
    /// Fee charged in base asset.
    pub fee_base: Uint128,
    /// Fee charged in quote asset.
    pub fee_quote: Uint128,
}

/// Clear the orders given a clearing price and volume.
pub fn fill_orders(
    bids: Vec<((Udec128, OrderId), LimitOrder)>,
    asks: Vec<((Udec128, OrderId), LimitOrder)>,
    clearing_price: Udec128,
    volume: Uint128,
    current_block_height: u64,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(bids.len() + asks.len());

    outcome.extend(fill_bids(
        bids,
        clearing_price,
        volume,
        current_block_height,
        maker_fee_rate,
        taker_fee_rate,
    )?);

    outcome.extend(fill_asks(
        asks,
        clearing_price,
        volume,
        current_block_height,
        maker_fee_rate,
        taker_fee_rate,
    )?);

    Ok(outcome)
}

/// Fill the BUY orders given a clearing price and volume.
fn fill_bids(
    bids: Vec<((Udec128, OrderId), LimitOrder)>,
    clearing_price: Udec128,
    mut volume: Uint128,
    current_block_height: u64,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(bids.len());

    for ((order_price, order_id), mut order) in bids {
        let filled = order.remaining.min(volume);

        order.remaining -= filled;
        volume -= filled;

        // Calculate fee based on whether the order is a maker or taker.
        //
        // An order is considered a maker, if it was created in an earlier block.
        // In other words, the liquidity already existed in the book at the
        // beginning of the current block.
        let fee_rate = if order.created_at_block_height < current_block_height {
            maker_fee_rate
        } else {
            taker_fee_rate
        };

        // For bids, the fee is paid in base asset.
        let fee_base = filled.checked_mul_dec_ceil(fee_rate)?;

        outcome.push(FillingOutcome {
            order_direction: Direction::Bid,
            order_price,
            order_id,
            order,
            filled,
            cleared: order.remaining.is_zero(),
            // Reduce the base refund by the fee amount.
            refund_base: filled.checked_sub(fee_base)?,
            // If the order is filled at a price better than the limit price,
            // we need to refund the trader the unused quote asset.
            refund_quote: filled.checked_mul_dec_floor(order_price - clearing_price)?,
            fee_base,
            fee_quote: Uint128::ZERO,
        });

        if volume.is_zero() {
            break;
        }
    }

    Ok(outcome)
}

/// Fill the SELL orders given a clearing price and volume.
fn fill_asks(
    asks: Vec<((Udec128, OrderId), LimitOrder)>,
    clearing_price: Udec128,
    mut volume: Uint128,
    current_block_height: u64,
    maker_fee: Udec128,
    taker_fee: Udec128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(asks.len());

    for ((order_price, order_id), mut order) in asks {
        let filled = order.remaining.min(volume);

        order.remaining -= filled;
        volume -= filled;

        // Calculate fee based on whether the order is a maker or taker.
        let fee_rate = if order.created_at_block_height < current_block_height {
            maker_fee
        } else {
            taker_fee
        };

        // For asks, the fee is paid in quote asset.
        let quote_amount = filled.checked_mul_dec_floor(clearing_price)?;
        let fee_quote = quote_amount.checked_mul_dec_ceil(fee_rate)?;

        outcome.push(FillingOutcome {
            order_direction: Direction::Ask,
            order_price,
            order_id,
            order,
            filled,
            cleared: order.remaining.is_zero(),
            refund_base: Uint128::ZERO,
            // Reduce the quote refund by the fee amount.
            refund_quote: quote_amount.checked_sub(fee_quote)?,
            fee_base: Uint128::ZERO,
            fee_quote,
        });

        if volume.is_zero() {
            break;
        }
    }

    Ok(outcome)
}
