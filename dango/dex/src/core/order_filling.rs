use {
    crate::{Order, OrderTrait},
    dango_types::dex::Direction,
    grug::{IsZero, NextNumber, Number, NumberConst, StdResult, Udec128, Udec256},
};

#[derive(Debug)]
pub struct FillingOutcome {
    pub order_direction: Direction,
    /// The order with the `filled` amount updated.
    pub order: Order,
    /// Amount this order was filled for, in base asset.
    pub filled_base: Udec256,
    /// Amount this order was filled for, in quote asset.
    pub filled_quote: Udec256,
    /// Amount of base asset that should be refunded to the trader.
    pub refund_base: Udec256,
    /// Amount of quote asset that should be refunded to the trader.
    pub refund_quote: Udec256,
    /// Fee charged in base asset.
    pub fee_base: Udec256,
    /// Fee charged in quote asset.
    pub fee_quote: Udec256,
}

/// Clear the orders given a clearing price and volume.
pub fn fill_orders(
    bids: Vec<(Udec256, Order)>,
    asks: Vec<(Udec256, Order)>,
    clearing_price: Udec256,
    volume: Udec256,
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
    bids: Vec<(Udec256, Order)>,
    clearing_price: Udec256,
    mut volume: Udec256,
    current_block_height: u64,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(bids.len());

    for (order_price, mut order) in bids {
        // Compute how much of the order can be filled.
        // This would be the order's remaining amount, or the remaining volume,
        // whichever is smaller.
        let filled_base = *order.remaining().min(&volume);
        let filled_quote = filled_base.checked_mul(clearing_price)?;

        // Deduct the amount filled from the order and the volume.
        order.fill(filled_base)?;
        volume -= filled_base;

        // Determine the fee rate for the limit order:
        // - if it's a passive order, it's not charged any fee;
        // - if it was created at a previous block height, then it's charged the maker fee rate;
        // - otherwise, it's charged the taker fee rate.
        let fee_rate = match order.created_at_block_height() {
            None => Udec128::ZERO,
            Some(block_height) if block_height < current_block_height => maker_fee_rate,
            Some(_) => taker_fee_rate,
        };

        // For bids, the fee is paid in base asset.
        let fee_base = filled_base.checked_mul(fee_rate.into_next())?;
        let fee_quote = Udec256::ZERO;

        // Determine the refund amounts.
        // For base, it's the filled amount minus the fee.
        // For quote, in case the order is filled at a price better than the
        // limit price, refund the unused deposit.
        let refund_base = filled_base.checked_sub(fee_base)?;
        let refund_quote = filled_base.checked_mul(order_price - clearing_price)?;

        outcome.push(FillingOutcome {
            order_direction: Direction::Bid,
            order,
            filled_base,
            filled_quote,
            refund_base,
            refund_quote,
            fee_base,
            fee_quote,
        });

        if volume.is_zero() {
            break;
        }
    }

    Ok(outcome)
}

/// Fill the SELL orders given a clearing price and volume.
fn fill_asks(
    asks: Vec<(Udec256, Order)>,
    clearing_price: Udec256,
    mut volume: Udec256,
    current_block_height: u64,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
) -> StdResult<Vec<FillingOutcome>> {
    let mut outcome = Vec::with_capacity(asks.len());

    for (_, mut order) in asks {
        // Compute how much of the order can be filled.
        // This would be the order's remaining amount, or the remaining volume,
        // whichever is smaller.
        let filled_base = *order.remaining().min(&volume);
        let filled_quote = filled_base.checked_mul(clearing_price)?;

        // Deduct the amount filled from the order and the volume.
        order.fill(filled_base)?;
        volume -= filled_base;

        // Calculate fee based on whether the order is a maker or taker.
        // Determine the fee rate for the limit order:
        // - if it's a passive order, it's not charged any fee;
        // - if it was created at a previous block height, then it's charged the maker fee rate;
        // - otherwise, it's charged the taker fee rate.
        let fee_rate = match order.created_at_block_height() {
            None => Udec128::ZERO,
            Some(block_height) if block_height < current_block_height => maker_fee_rate,
            Some(_) => taker_fee_rate,
        };

        // For asks, the fee is paid in quote asset.
        let fee_base = Udec256::ZERO;
        let fee_quote = filled_quote.checked_mul(fee_rate.into_next())?;

        // Determine the refund amounts.
        // For base, since limit orders are good-till-canceled, no need to refund.
        // For quote, it's the filled amount minus the fee.
        let refund_base = Udec256::ZERO;
        let refund_quote = filled_quote.checked_sub(fee_quote)?;

        outcome.push(FillingOutcome {
            order_direction: Direction::Ask,
            order,
            filled_base,
            filled_quote,
            refund_base,
            refund_quote,
            fee_base,
            fee_quote,
        });

        if volume.is_zero() {
            break;
        }
    }

    Ok(outcome)
}
