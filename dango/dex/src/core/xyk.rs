use {
    crate::PassiveOrder,
    grug::{
        Bounded, CoinPair, IsZero, MathResult, MultiplyFraction, MultiplyRatio, Number,
        NumberConst, Udec128, Uint128, ZeroExclusiveOneExclusive,
    },
    std::{cmp, iter},
};

pub fn add_initial_liquidity(deposit: &CoinPair) -> MathResult<Uint128> {
    normalized_invariant(deposit)
}

pub fn add_subsequent_liquidity(
    reserve: &mut CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<Udec128> {
    let invariant_before = normalized_invariant(reserve)?;

    // Add the used funds to the pool reserves.
    reserve.merge(deposit)?;

    // Compute the proportional increase in the invariant.
    let invariant_after = normalized_invariant(reserve)?;
    let invariant_ratio = Udec128::checked_from_ratio(invariant_after, invariant_before)?;

    // Compute the mint ratio from the invariant ratio based on the curve type.
    // This ensures that an unbalances provision will be equivalent to a swap
    // followed by a balancedliquidity provision.
    Ok(invariant_ratio.checked_sub(Udec128::ONE)?)
}

pub fn swap_exact_amount_in(
    input_amount: Uint128,
    input_reserve: Uint128,
    output_reserve: Uint128,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> MathResult<Uint128> {
    // Solve A * B = (A + input_amount) * (B - output_amount) for output_amount
    // => output_amount = B - (A * B) / (A + input_amount)
    // Round so that user takes the loss.
    let output_amount =
        output_reserve.checked_sub(input_reserve.checked_multiply_ratio_ceil(
            output_reserve,
            input_reserve.checked_add(input_amount)?,
        )?)?;

    // Apply swap fee. Round so that user takes the loss.
    output_amount.checked_mul_dec_floor(Udec128::ONE - *swap_fee_rate)
}

pub fn swap_exact_amount_out(
    output_amount: Uint128,
    input_reserve: Uint128,
    output_reserve: Uint128,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> MathResult<Uint128> {
    // Apply swap fee. In SwapExactIn we multiply ask by (1 - fee) to get the
    // offer amount after fees. So in this case we need to divide ask by (1 - fee)
    // to get the ask amount after fees.
    // Round so that user takes the loss.
    let output_amount_before_fee =
        output_amount.checked_div_dec_ceil(Udec128::ONE - *swap_fee_rate)?;

    // Solve A * B = (A + input_amount) * (B - output_amount) for input_amount
    // => input_amount = (A * B) / (B - output_amount) - A
    // Round so that user takes the loss.
    Uint128::ONE
        .checked_multiply_ratio_floor(
            input_reserve.checked_mul(output_reserve)?,
            output_reserve.checked_sub(output_amount_before_fee)?,
        )?
        .checked_sub(input_reserve)
}

pub fn reflect_curve(
    base_reserve: Uint128,
    quote_reserve: Uint128,
    order_spacing: Udec128,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> anyhow::Result<(
    Box<dyn Iterator<Item = (Udec128, PassiveOrder)>>,
    Box<dyn Iterator<Item = (Udec128, PassiveOrder)>>,
)> {
    // Compute the marginal price. We will place orders above and below this price.
    let marginal_price = Udec128::checked_from_ratio(quote_reserve, base_reserve)?;

    // Construct the bid order iterator.
    // Start from the marginal price minus the swap fee rate.
    let bids = {
        let mut id = 0;
        let one_sub_fee_rate = Udec128::ONE.checked_sub(*swap_fee_rate)?;
        let mut maybe_price = marginal_price.checked_mul(one_sub_fee_rate).ok();
        let mut prev_size = Uint128::ZERO;
        let mut prev_size_quote = Uint128::ZERO;

        iter::from_fn(move || {
            // Terminate if price is less or equal to zero.
            let price = match maybe_price {
                Some(price) if price.is_non_zero() => price,
                _ => return None,
            };

            // Compute the total order size (in base asset) at this price.
            let quote_reserve_div_price = quote_reserve.checked_div_dec(price).ok()?;
            let mut size = quote_reserve_div_price.checked_sub(base_reserve).ok()?;

            // Compute the order size (in base asset) at this price.
            //
            // This is the difference between the total order size at
            // this price, and that at the previous price.
            let mut amount = size.checked_sub(prev_size).ok()?;

            // Compute the total order size (in quote asset) at this price.
            let mut amount_quote = amount.checked_mul_dec_ceil(price).ok()?;
            let mut size_quote = prev_size_quote.checked_add(amount_quote).ok()?;

            // If total order size (in quote asset) is greater than the
            // reserve, cap it to the reserve size.
            if size_quote > quote_reserve {
                size_quote = quote_reserve;
                amount_quote = size_quote.checked_sub(prev_size_quote).ok()?;
                amount = amount_quote.checked_div_dec_floor(price).ok()?;
                size = prev_size.checked_add(amount).ok()?;
            }

            // If order size is zero, we have ran out of liquidity.
            // Terminate the iterator.
            if amount.is_zero() {
                return None;
            }

            // Update the iterator state.
            id += 1;
            prev_size = size;
            prev_size_quote = size_quote;
            maybe_price = price.checked_sub(order_spacing).ok();

            Some((price, PassiveOrder {
                id,
                price,
                amount,
                remaining: amount,
            }))
        })
    };

    // Construct the ask order iterator.
    let asks = {
        let mut id = u64::MAX;
        let one_plus_fee_rate = Udec128::ONE.checked_add(*swap_fee_rate)?;
        let mut maybe_price = marginal_price.checked_mul(one_plus_fee_rate).ok();
        let mut prev_size = Uint128::ZERO;

        iter::from_fn(move || {
            let price = maybe_price?;

            // Compute the total order size (in base asset) at this price.
            let quote_reserve_div_price = quote_reserve.checked_div_dec(price).ok()?;
            let size = base_reserve.checked_sub(quote_reserve_div_price).ok()?;

            // If total order size (in base asset) exceeds the base asset
            // reserve, cap it to the reserve size.
            let size = cmp::min(size, base_reserve);

            // Compute the order size (in base asset) at this price.
            //
            // This is the difference between the total order size at
            // this price, and that at the previous price.
            let amount = size.checked_sub(prev_size).ok()?;

            // If order size is zero, we have ran out of liquidity.
            // Terminate the iterator.
            if amount.is_zero() {
                return None;
            }

            // Update the iterator state.
            id -= 1;
            prev_size = size;
            maybe_price = price.checked_add(order_spacing).ok();

            Some((price, PassiveOrder {
                id,
                price,
                amount,
                remaining: amount,
            }))
        })
    };

    Ok((Box::new(bids), Box::new(asks)))
}

/// Compute `sqrt(A * B)`, where `A` and `B` are the reserve amount of the two
/// assets in an xyk pool.
pub fn normalized_invariant(reserve: &CoinPair) -> MathResult<Uint128> {
    let a = *reserve.first().amount;
    let b = *reserve.second().amount;

    a.checked_mul(b)?.checked_sqrt()
}
