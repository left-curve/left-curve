use {
    crate::PassiveOrder,
    grug::{
        Bounded, CoinPair, IsZero, MathResult, MultiplyFraction, MultiplyRatio, NextNumber, Number,
        NumberConst, PrevNumber, Udec128, Udec256, Uint64, Uint128, ZeroExclusiveOneExclusive,
        ZeroInclusiveOneExclusive,
    },
    std::{cmp, iter},
};

const INITIAL_LP_TOKEN_MULTIPLIER: Uint128 = Uint128::new(1_000_000u128);

pub fn add_initial_liquidity(deposit: &CoinPair) -> MathResult<Uint128> {
    normalized_invariant(deposit)?.checked_mul(INITIAL_LP_TOKEN_MULTIPLIER)
}

pub fn add_subsequent_liquidity(
    reserve: &mut CoinPair,
    deposit: CoinPair,
) -> anyhow::Result<Udec256> {
    let invariant_before = normalized_invariant(reserve)?;

    // Add the used funds to the pool reserves.
    reserve.merge(deposit)?;

    // Compute the proportional increase in the invariant.
    let invariant_after = normalized_invariant(reserve)?;
    let invariant_ratio = Udec128::checked_from_ratio(invariant_after, invariant_before)?;

    // Compute the mint ratio from the invariant ratio based on the curve type.
    // This ensures that an unbalances provision will be equivalent to a swap
    // followed by a balancedliquidity provision.
    Ok(invariant_ratio.checked_sub(Udec128::ONE)?.into_next())
}

/// Note: this function does not concern the liquidity fee.
/// Liquidity fee logics are found in `PairParams::swap_exact_amount_in`, in `liquidity_pool.rs`.
pub fn swap_exact_amount_in(
    input_reserve: Uint128,
    output_reserve: Uint128,
    input_amount: Uint128,
) -> MathResult<Uint128> {
    // Solve A * B = (A + input_amount) * (B - output_amount) for output_amount
    // => output_amount = B - (A * B) / (A + input_amount)
    // Round so that user takes the loss.
    output_reserve.checked_sub(
        input_reserve.checked_multiply_ratio_ceil(
            output_reserve,
            input_reserve.checked_add(input_amount)?,
        )?,
    )
}

/// Note: this function does not concern the liquidity fee.
/// Liquidity fee logics are found in `PairParams::swap_exact_amount_out`, in `liquidity_pool.rs`.
pub fn swap_exact_amount_out(
    input_reserve: Uint128,
    output_reserve: Uint128,
    output_amount: Uint128,
) -> MathResult<Uint128> {
    // Solve A * B = (A + input_amount) * (B - output_amount) for input_amount
    // => input_amount = (A * B) / (B - output_amount) - A
    // Round so that user takes the loss.
    Uint128::ONE
        .checked_multiply_ratio_floor(
            input_reserve.checked_mul(output_reserve)?,
            output_reserve.checked_sub(output_amount)?,
        )?
        .checked_sub(input_reserve)
}

pub fn reflect_curve(
    mut base_reserve: Uint128,
    mut quote_reserve: Uint128,
    order_spacing: Udec128,
    reserve_ratio: Bounded<Udec128, ZeroInclusiveOneExclusive>,
    swap_fee_rate: Bounded<Udec128, ZeroExclusiveOneExclusive>,
) -> anyhow::Result<(
    Box<dyn Iterator<Item = (Udec256, PassiveOrder)>>,
    Box<dyn Iterator<Item = (Udec256, PassiveOrder)>>,
)> {
    // Withhold the funds corresponding to the reserve requirement.
    // These funds will not be used to place orders.
    let one_sub_reserve_ratio = Udec128::ONE - *reserve_ratio;
    base_reserve.checked_mul_dec_floor_assign(one_sub_reserve_ratio)?;
    quote_reserve.checked_mul_dec_floor_assign(one_sub_reserve_ratio)?;

    // Compute the marginal price. We will place orders above and below this price.
    let marginal_price =
        Udec256::checked_from_ratio(quote_reserve.into_next(), base_reserve.into_next())?;

    // Construct the bid order iterator.
    // Start from the marginal price minus the swap fee rate.
    let bids = {
        let mut id = Uint64::ZERO;
        let one_sub_fee_rate = Udec128::ONE.checked_sub(*swap_fee_rate)?;
        let mut maybe_price = marginal_price
            .checked_mul(one_sub_fee_rate.into_next())
            .ok();
        let mut prev_size = Uint128::ZERO;
        let mut prev_size_quote = Uint128::ZERO;

        iter::from_fn(move || {
            // Terminate if price is less or equal to zero.
            let price = match maybe_price {
                Some(price) if price.is_non_zero() => price,
                _ => return None,
            };

            // Compute the total order size (in base asset) at this price.
            let quote_reserve_div_price = quote_reserve
                .into_next()
                .checked_div_dec(price)
                .ok()?
                .checked_into_prev()
                .ok()?;
            let mut size = quote_reserve_div_price.checked_sub(base_reserve).ok()?;

            // Compute the order size (in base asset) at this price.
            //
            // This is the difference between the total order size at
            // this price, and that at the previous price.
            let mut amount = size.checked_sub(prev_size).ok()?;

            // Compute the total order size (in quote asset) at this price.
            let mut amount_quote = amount
                .into_next()
                .checked_mul_dec_ceil(price)
                .ok()?
                .checked_into_prev()
                .ok()?;
            let mut size_quote = prev_size_quote.checked_add(amount_quote).ok()?;

            // If total order size (in quote asset) is greater than the
            // reserve, cap it to the reserve size.
            if size_quote > quote_reserve {
                size_quote = quote_reserve;
                amount_quote = size_quote.checked_sub(prev_size_quote).ok()?;
                amount = amount_quote
                    .into_next()
                    .checked_div_dec_floor(price)
                    .ok()?
                    .checked_into_prev()
                    .ok()?;
                size = prev_size.checked_add(amount).ok()?;
            }

            // If order size is zero, we have ran out of liquidity.
            // Terminate the iterator.
            if amount.is_zero() {
                return None;
            }

            // Update the iterator state.
            id += Uint64::ONE;
            prev_size = size;
            prev_size_quote = size_quote;
            maybe_price = price.checked_sub(order_spacing.into_next()).ok();

            Some((price, PassiveOrder {
                id,
                price,
                amount,
                remaining: amount.into_next().checked_into_dec().ok()?,
            }))
        })
    };

    // Construct the ask order iterator.
    let asks = {
        let mut id = Uint64::MAX;
        let one_plus_fee_rate = Udec256::ONE.checked_add(swap_fee_rate.into_next())?;
        let mut maybe_price = marginal_price.checked_mul(one_plus_fee_rate).ok();
        let mut prev_size = Uint128::ZERO;

        iter::from_fn(move || {
            let price = maybe_price?;

            // Compute the total order size (in base asset) at this price.
            let quote_reserve_div_price = quote_reserve
                .into_next()
                .checked_div_dec(price)
                .ok()?
                .checked_into_prev()
                .ok()?;
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
            id -= Uint64::ONE;
            prev_size = size;
            maybe_price = price.checked_add(order_spacing.into_next()).ok();

            Some((price, PassiveOrder {
                id,
                price,
                amount,
                remaining: amount.into_next().checked_into_dec().ok()?,
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
