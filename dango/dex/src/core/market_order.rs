use {
    crate::{ExtendedOrderId, FillingOutcome, MarketOrder, Order, OrderTrait},
    dango_types::dex::{Direction, OrderId},
    grug::{MultiplyFraction, Number, NumberConst, Signed, StdResult, Udec128, Uint128, Unsigned},
    std::{cmp::Ordering, collections::HashMap, iter::Peekable},
};

/// Match and fill market orders against the limit order book.
///
/// Returns a tuple containing the filling outcomes.
pub fn match_and_fill_market_orders<M, L>(
    market_orders: &mut Peekable<M>,
    limit_orders: &mut Peekable<L>,
    market_order_direction: Direction,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    current_block_height: u64,
) -> anyhow::Result<HashMap<ExtendedOrderId, FillingOutcome>>
where
    M: Iterator<Item = (OrderId, MarketOrder)>,
    L: Iterator<Item = StdResult<(Udec128, Order)>>,
{
    let mut filling_outcomes = HashMap::new();

    // Match the market order to the opposite side of the resting limit order book.
    let limit_order_direction = -market_order_direction;

    // Find the best offer price in the resting limit order book.
    // This will be used to compute the market order's worst average execution
    // price, based on its max slippage.
    let best_price = match limit_orders.peek_mut() {
        Some(Ok((price, _))) => *price,
        Some(Err(e)) => return Err(e.clone().into()),
        None => return Ok(filling_outcomes), // Return early if there are no limit orders
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
        let (price, limit_order) = match limit_orders.peek_mut() {
            Some(Ok((price, ref mut limit_order))) => (price, limit_order),
            Some(Err(e)) => return Err(e.clone().into()),
            None => break,
        };

        let Some((market_order_id, market_order)) = market_orders.peek_mut() else {
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

        let market_order_amount_in_base = match market_order_direction {
            Direction::Bid => market_order.remaining.checked_div_dec_floor(*price)?,
            Direction::Ask => market_order.remaining,
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
            let extended_market_order_id = ExtendedOrderId::User(*market_order_id);
            let filling_outcome = filling_outcomes.get_mut(&extended_market_order_id).unwrap();

            // Calculate how much of the market order can be filled without the average
            // price of the market order exceeding the cutoff price.
            // TODO: optimize the math here. See the jupyter notebook.
            let current_avg_price = Udec128::checked_from_ratio(
                filling_outcome.filled_quote,
                filling_outcome.filled_base,
            )?;
            let price_ratio = current_avg_price
                .checked_into_signed()?
                .checked_sub(cutoff_price.checked_into_signed()?)?
                .checked_div(
                    cutoff_price
                        .checked_into_signed()?
                        .checked_sub(price.checked_into_signed()?)?,
                )?;
            let market_order_amount_to_match_in_base = filling_outcome
                .filled_base
                .checked_mul_dec_floor(price_ratio.checked_into_unsigned()?)?
                .min(market_order_amount_in_base);

            // Since the order is only partially filled we update the filling outcome
            // to refund the amount that was not filled.
            match market_order_direction {
                Direction::Bid => {
                    filling_outcome.refund_quote.checked_add_assign(
                        market_order.remaining.checked_sub(
                            market_order_amount_to_match_in_base.checked_mul_dec_ceil(*price)?,
                        )?,
                    )?;
                },
                Direction::Ask => {
                    filling_outcome.refund_base.checked_add_assign(
                        market_order
                            .remaining
                            .checked_sub(market_order_amount_to_match_in_base)?,
                    )?;
                },
            }

            market_order_amount_to_match_in_base
        };

        // If the amount to match is zero, skip this market order as it cannot be filled.
        // We do not refund the market order since that would allow spamming the contract with
        // tiny market orders at no cost.
        if market_order_amount_to_match_in_base.is_zero() {
            market_orders.next();
            continue;
        }

        // If the resulting output of the match for a SELL market order is zero,
        // we skip it because it cannot be filled.
        match market_order_direction {
            Direction::Ask
                if market_order_amount_to_match_in_base
                    .checked_mul_dec_floor(*price)?
                    .is_zero() =>
            {
                market_orders.next();
                continue;
            },
            _ => {},
        }

        // For a market ASK order the amount is in terms of the base asset. So we can directly
        // match it against the limit order remaining amount
        let (filled_base, price, limit_order, market_order) =
            match market_order_amount_to_match_in_base.cmp(limit_order.remaining()) {
                // The market ask order is smaller than the limit order so we advance the market
                // orders iterator and decrement the limit order remaining amount
                Ordering::Less => {
                    limit_order.fill(market_order_amount_to_match_in_base)?;
                    market_order.clear();

                    // Clone values so we can next the market order iterator
                    let return_tuple = (
                        market_order_amount_to_match_in_base,
                        *price,
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
                    limit_order.clear();
                    market_order.clear();

                    // Clone values so we can next the limit order iterator
                    let return_tuple = (
                        market_order_amount_to_match_in_base,
                        *price,
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
                    let fill_amount = limit_order.clear();

                    // Decrement the market order amount by the limit order remaining amount.
                    // This is done differently for BUY and SELL market orders because the amount
                    // is in terms of the quote asset for BUY orders and in terms of the base asset
                    // for SELL orders.
                    // If this is the last market order to be matched, i.e. the limit order iterator
                    // is exhausted, the market order will remain in the market orders iterator and
                    // the amount left in the market order will be refunded in `cron_execute`.
                    match market_order_direction {
                        Direction::Bid => {
                            let fill_amount_in_quote = fill_amount.checked_mul_dec_ceil(*price)?;
                            market_order.fill(fill_amount_in_quote)?;
                        },
                        Direction::Ask => {
                            market_order.fill(fill_amount)?;
                        },
                    }

                    let return_tuple = (fill_amount, *price, *limit_order, *market_order);

                    // Pop the limits iterator
                    limit_orders.next();

                    return_tuple
                },
            };

        // Determine the fee rate for the limit order:
        // - if it's a passive order, it's not charged any fee;
        // - if it was created at a previous block height, then it's charged the maker fee rate;
        // - otherwise, it's charged the taker fee rate.
        let limit_order_fee_rate = match limit_order.created_at_block_height() {
            None => Udec128::ZERO,
            Some(block_height) if block_height < current_block_height => maker_fee_rate,
            Some(_) => taker_fee_rate,
        };

        update_filling_outcome(
            &mut filling_outcomes,
            limit_order,
            limit_order_direction,
            filled_base,
            price,
            limit_order_fee_rate,
        )?;

        update_filling_outcome(
            &mut filling_outcomes,
            Order::Market(market_order),
            market_order_direction,
            filled_base,
            price,
            taker_fee_rate,
        )?;
    }

    Ok(filling_outcomes)
}

fn update_filling_outcome(
    filling_outcomes: &mut HashMap<ExtendedOrderId, FillingOutcome>,
    order: Order,
    order_direction: Direction,
    filled_base: Uint128,
    price: Udec128,
    fee_rate: Udec128,
) -> StdResult<()> {
    let filling_outcome = filling_outcomes
        .entry(order.extended_id())
        .or_insert_with(|| FillingOutcome {
            order_direction,
            order,
            filled_base: Uint128::ZERO,
            filled_quote: Uint128::ZERO,
            refund_base: Uint128::ZERO,
            refund_quote: Uint128::ZERO,
            fee_base: Uint128::ZERO,
            fee_quote: Uint128::ZERO,
        });

    let filled_quote = filled_base.checked_mul_dec_floor(price)?;

    filling_outcome
        .filled_base
        .checked_add_assign(filled_base)?;
    filling_outcome
        .filled_quote
        .checked_add_assign(filled_base.checked_mul_dec_floor(price)?)?;
    filling_outcome.order = order;

    match order_direction {
        Direction::Bid => {
            let fee_base = filled_base.checked_mul_dec_ceil(fee_rate)?;

            filling_outcome.fee_base.checked_add_assign(fee_base)?;
            filling_outcome
                .refund_base
                .checked_add_assign(filled_base.checked_sub(fee_base)?)?;
        },
        Direction::Ask => {
            let fee_quote = filled_quote.checked_mul_dec_ceil(fee_rate)?;

            filling_outcome.fee_quote.checked_add_assign(fee_quote)?;
            filling_outcome
                .refund_quote
                .checked_add_assign(filled_quote.checked_sub(fee_quote)?)?;
        },
    }

    Ok(())
}
