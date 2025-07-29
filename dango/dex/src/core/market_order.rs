use {
    crate::{ExtendedOrderId, FillingOutcome, MarketOrder, Order, OrderTrait},
    dango_types::dex::{Direction, OrderId},
    grug::{
        IsZero, Number, NumberConst, Signed, StdResult, Udec128, Udec128_6, Udec128_24, Unsigned,
    },
    std::{cmp::Ordering, collections::HashMap},
};

/// Match and fill market orders against the limit order book.
///
/// ## Returns
///
/// - A map containing the filling outcomes (for both market and limit orders).
/// - In case a market order is left partially filled, it is returned, so the
///   refund can be handled properly.
/// - Similarly, in case a limit order is left partially filled, it is returned,
///   so it can later be used in limit order matching.
pub fn match_and_fill_market_orders<M, L>(
    market_orders: &mut M,
    limit_orders: &mut L,
    market_order_direction: Direction,
    maker_fee_rate: Udec128,
    taker_fee_rate: Udec128,
    current_block_height: u64,
) -> anyhow::Result<(
    HashMap<ExtendedOrderId, FillingOutcome>,
    Option<(OrderId, MarketOrder)>,
    Option<(Udec128_24, Order)>,
)>
where
    M: Iterator<Item = (OrderId, MarketOrder)>,
    L: Iterator<Item = StdResult<(Udec128_24, Order)>>,
{
    let mut current_market_order = market_orders.next();
    let mut current_limit_order = limit_orders.next().transpose()?;
    let mut current_best_price = current_limit_order.map(|(price, _)| price);
    let mut filling_outcomes = HashMap::<ExtendedOrderId, FillingOutcome>::new();

    // Match the market order to the opposite side of the resting limit order book.
    let limit_order_direction = -market_order_direction;

    // Iterate over the limit orders and market orders until one of them is exhausted.
    // Since a market order can partially fill a limit order, and that limit order should
    // remain in the book partially filled, we mutably peek the limit orders iterator and
    // only advance it when the market order amount is greater than or equal to the remaining
    // amount of the limit order.
    //
    // This is not the case for the market orders. They are matched in the order they were
    // received, and do not remain after matching is completed.
    loop {
        let Some((price, limit_order)) = current_limit_order.as_mut() else {
            break;
        };

        let Some((market_order_id, market_order)) = current_market_order.as_mut() else {
            break;
        };

        let Some(best_price) = current_best_price else {
            break;
        };

        // Calculate the cutoff price for the current market order
        let cutoff_price = match market_order_direction {
            Direction::Bid => Udec128_24::ONE
                .saturating_add(market_order.max_slippage)
                .saturating_mul(best_price),
            Direction::Ask => Udec128_24::ONE
                .saturating_sub(market_order.max_slippage)
                .saturating_mul(best_price),
        };

        // The direction of the comparison depends on whether the market order
        // is a BUY or a SELL.
        let price_is_worse_than_cutoff = match market_order_direction {
            Direction::Bid => *price > cutoff_price,
            Direction::Ask => *price < cutoff_price,
        };

        let market_order_amount_in_base = match market_order_direction {
            Direction::Bid => market_order.remaining.checked_div(*price)?,
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
        let filled_base = if !price_is_worse_than_cutoff {
            market_order_amount_in_base
        } else {
            let extended_market_order_id = ExtendedOrderId::User(*market_order_id);
            let filling_outcome = filling_outcomes.get_mut(&extended_market_order_id).unwrap();

            // Calculate how much of the market order can be filled without the average
            // price of the market order exceeding the cutoff price.
            // TODO: optimize the math here. See the jupyter notebook.

            // This is wrong and it makes the crongboj fails!

            // let current_avg_price = filling_outcome
            //     .filled_quote
            //     .checked_div(filling_outcome.filled_base)?
            //     .convert_precision::<24>()?; // TODO: Use other precision for amounts?

            // This is the correct way to calculate the current average price.
            let current_avg_price = Udec128_24::checked_from_ratio(
                filling_outcome.filled_quote.0,
                filling_outcome.filled_base.0,
            )?;

            println!(
                "filled_quote: {} | filled_base: {} | current_avg_price: {current_avg_price} | cutoff: {cutoff_price} | price: {price} | best_price: {best_price} | slippage: {} | direction: {market_order_direction:?}",
                filling_outcome.filled_quote,
                filling_outcome.filled_base,
                market_order.max_slippage
            );

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
                .checked_mul(price_ratio.checked_into_unsigned()?)?
                .min(market_order_amount_in_base);

            // Since the order is only partially filled we update the filling outcome
            // to refund the amount that was not filled.
            match market_order_direction {
                Direction::Bid => {
                    filling_outcome.refund_quote.checked_add_assign(
                        market_order.remaining.checked_sub(
                            market_order_amount_to_match_in_base.checked_mul(*price)?,
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

        // If the amount to match is zero, skip this market order as it cannot
        // be filled.
        // We do not refund the market order since that would allow spamming the
        // contract with tiny market orders at no cost.
        if filled_base.is_zero() {
            market_orders.next();
            continue;
        }

        // If the resulting output of the match for a SELL market order is zero,
        // we skip it because it cannot be filled.
        if market_order_direction == Direction::Ask {
            let filled_quote = filled_base.checked_mul(*price)?;
            if filled_quote.is_zero() {
                market_orders.next();
                continue;
            }
        }

        // For a market ASK order the amount is in terms of the base asset. So we can directly
        // match it against the limit order remaining amount
        let (filled_base, price, limit_order, market_order) =
            match filled_base.cmp(limit_order.remaining()) {
                // The market ask order is smaller than the limit order so we advance the market
                // orders iterator and decrement the limit order remaining amount
                Ordering::Less => {
                    limit_order.fill(filled_base)?;
                    market_order.clear();

                    // Clone values so we can next the market order iterator
                    let return_tuple = (filled_base, *price, *limit_order, *market_order);

                    // Advance the market orders iterator
                    current_market_order = market_orders.next();

                    // Set the best price to the current limit order's price.
                    current_best_price = Some(*price);

                    return_tuple
                },
                // The market order amount is equal to the limit order remaining amount, so we can
                // match both in full, and advance both iterators.
                Ordering::Equal => {
                    limit_order.clear();
                    market_order.clear();

                    // Clone values so we can next the limit order iterator
                    let return_tuple = (filled_base, *price, *limit_order, *market_order);

                    // Advance the both order iterators
                    current_limit_order = limit_orders.next().transpose()?;
                    current_market_order = market_orders.next();

                    // Set the best price to the new limit order's price.
                    current_best_price = current_limit_order.map(|(price, _)| price);

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
                            let fill_amount_in_quote = fill_amount.checked_mul(*price)?;
                            market_order.fill(fill_amount_in_quote)?;
                        },
                        Direction::Ask => {
                            market_order.fill(fill_amount)?;
                        },
                    }

                    let return_tuple = (fill_amount, *price, *limit_order, *market_order);

                    // Pop the limits iterator
                    current_limit_order = limit_orders.next().transpose()?;

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
            taker_fee_rate, // A market order is always a taker.
        )?;
    }

    let left_over_market_order = match current_market_order {
        Some((id, order)) if order.remaining.is_non_zero() => Some((id, order)),
        _ => None,
    };

    let left_over_limit_order = match current_limit_order {
        Some((price, order)) if order.remaining().is_non_zero() => Some((price, order)),
        _ => None,
    };

    Ok((
        filling_outcomes,
        left_over_market_order,
        left_over_limit_order,
    ))
}

fn update_filling_outcome(
    filling_outcomes: &mut HashMap<ExtendedOrderId, FillingOutcome>,
    order: Order,
    order_direction: Direction,
    filled_base: Udec128_6,
    price: Udec128_24,
    fee_rate: Udec128,
) -> StdResult<()> {
    let filling_outcome = filling_outcomes
        .entry(order.extended_id())
        .or_insert_with(|| FillingOutcome {
            order_direction,
            order,
            filled_base: Udec128_6::ZERO,
            filled_quote: Udec128_6::ZERO,
            refund_base: Udec128_6::ZERO,
            refund_quote: Udec128_6::ZERO,
            fee_base: Udec128_6::ZERO,
            fee_quote: Udec128_6::ZERO,
        });

    let filled_quote = filled_base.checked_mul(price)?;

    filling_outcome
        .filled_base
        .checked_add_assign(filled_base)?;
    filling_outcome
        .filled_quote
        .checked_add_assign(filled_base.checked_mul(price)?)?;
    filling_outcome.order = order;

    match order_direction {
        Direction::Bid => {
            let fee_base = filled_base.checked_mul(fee_rate)?;

            filling_outcome.fee_base.checked_add_assign(fee_base)?;
            filling_outcome
                .refund_base
                .checked_add_assign(filled_base.checked_sub(fee_base)?)?;
        },
        Direction::Ask => {
            let fee_quote = filled_quote.checked_mul(fee_rate)?;

            filling_outcome.fee_quote.checked_add_assign(fee_quote)?;
            filling_outcome
                .refund_quote
                .checked_add_assign(filled_quote.checked_sub(fee_quote)?)?;
        },
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::LimitOrder,
        grug::{Addr, Uint128},
    };

    /// Two SELL limit orders, one with a price significantly lower than the other.
    /// Two BUY market orders.
    ///
    /// Market order #1 exactly consumes limit order #1. Market order #2 partially
    /// consumes limit order #2.
    ///
    /// Under the old logic, the fact that 1) limit order #1 has a price significantly
    /// lower than limit order #2, and 2) it's consumed exactly, triggers an edge
    /// case where the code panics on this `unwrap` statement:
    ///
    /// ```rust
    /// let filling_outcome = filling_outcomes.get_mut(&extended_market_order_id).unwrap();
    /// ```
    ///
    /// A better fix is to not use `unwrap` at all, but we don't have enough time
    /// for a large refactor like that, so for now we do a quick and easy fix,
    /// to ensure it at least doesn't panic.
    #[test]
    fn panic_case_found_on_testnet() {
        let mut limit_orders = [
            Ok((
                Udec128_24::new(10),
                Order::Limit(LimitOrder {
                    user: Addr::mock(1),
                    id: OrderId::new(1),
                    price: Udec128_24::new(10),
                    amount: Uint128::new(1),
                    remaining: Udec128_6::new(1),
                    created_at_block_height: 0,
                }),
            )),
            Ok((
                Udec128_24::new(1000),
                Order::Limit(LimitOrder {
                    user: Addr::mock(2),
                    id: OrderId::new(2),
                    price: Udec128_24::new(1000),
                    amount: Uint128::new(2),
                    remaining: Udec128_6::new(2),
                    created_at_block_height: 0,
                }),
            )),
        ]
        .into_iter()
        .peekable();

        let mut market_orders = [
            (OrderId::new(3), MarketOrder {
                user: Addr::mock(3),
                id: OrderId::new(3),
                amount: Uint128::new(10), /* base amount 1 * price 10, should exactly consume limit order 1 */
                remaining: Udec128_6::new(10),
                max_slippage: Udec128::ZERO,
            }),
            (OrderId::new(4), MarketOrder {
                user: Addr::mock(4),
                id: OrderId::new(4),
                amount: Uint128::new(1000), // should consume 1 of the 2 base tokens for sale by limit order #2
                remaining: Udec128_6::new(1000),
                max_slippage: Udec128::ZERO,
            }),
        ]
        .into_iter()
        .peekable();

        // With the old logic, this function call panics. Here we just ensure it
        // doesn't panic, and 4 filling outcomes are returned.
        let (outcomes, left_over_market_order, left_over_limit_order) =
            match_and_fill_market_orders(
                &mut market_orders,
                &mut limit_orders,
                Direction::Bid,
                Udec128::ZERO,
                Udec128::ZERO,
                0,
            )
            .unwrap();
        assert_eq!(outcomes.len(), 4);
        assert!(left_over_market_order.is_none());
        assert!(
            left_over_limit_order.is_some_and(|(_, order)| *order.remaining() == Udec128_6::new(1))
        );
    }
}
