use {
    crate::DEPTHS,
    dango_types::dex::{Direction, Price},
    grug::{Denom, Fraction, IsZero, MathResult, NonZero, Number, StdResult, Storage, Udec128_6},
    std::collections::BTreeSet,
};

/// For bids, return the bucket that is immediately smaller than the price.
/// For asks, return the bucket that is immediately larger than the price.
pub fn get_bucket(bucket_size: Price, direction: Direction, price: Price) -> MathResult<Price> {
    // This is the bucket immediately smaller than the price.
    let lower = {
        // Note: instead of the decimal, we work with the integer numerators of
        // the price and bucket size. This minimizes the number of mul/div
        // operations (which are expensive) and also, more importantly,
        // eliminates a possible overflow (see the test cases named "no overflow"
        // at the bottom of this file).
        let numerator = price
            .numerator()
            .checked_div(bucket_size.numerator())?
            .checked_mul(bucket_size.numerator())?;
        Price::raw(numerator)
    };

    debug_assert!(
        lower <= price,
        "lower bucket ({lower}) is somehow bigger than the price ({price})"
    );

    // If the order is an ask, and the price isn't exactly equal the bucket,
    // then return one bucket larger.
    // Otherwise, return the bucket.
    match direction {
        Direction::Ask if price > lower => Ok(lower.saturating_add(bucket_size)),
        _ => Ok(lower),
    }
}

/// Increase the liquidity depths of the given bucket sizes.
///
/// This is called in two circumstances:
/// - in `execute::batch_update_orders`, when creating new user limit orders;
/// - in `cron::auction`, when creating new passive orders.
pub fn increase_liquidity_depths(
    storage: &mut dyn Storage,
    base_denom: &Denom,
    quote_denom: &Denom,
    direction: Direction,
    price: Price,
    amount_base: Udec128_6,
    bucket_sizes: &BTreeSet<NonZero<Price>>,
) -> StdResult<()> {
    let amount_quote = amount_base.checked_mul(price)?;

    for bucket_size in bucket_sizes {
        let bucket = get_bucket(**bucket_size, direction, price)?;
        let key = ((base_denom, quote_denom), **bucket_size, direction, bucket);

        DEPTHS.may_update(storage, key, |maybe_depths| -> StdResult<_> {
            let (mut depth_base, mut depth_quote) = maybe_depths.unwrap_or_default();

            depth_base.checked_add_assign(amount_base)?;
            depth_quote.checked_add_assign(amount_quote)?;

            Ok((depth_base, depth_quote))
        })?;
    }

    Ok(())
}

/// Decrease the liquidity depths of the given bucket sizes.
///
///
///
/// This is called under three circumstances:
/// - in `execute::batch_update_orders`, when canceling user limit orders;
/// - in `cron::auction`, when canceling passive orders from the previous block;
/// - in `cron::clear_orders_of_pair`, when a limit order (user or passive) is fulfilled.
pub fn decrease_liquidity_depths(
    storage: &mut dyn Storage,
    base_denom: &Denom,
    quote_denom: &Denom,
    direction: Direction,
    price: Price,
    amount_base: Udec128_6,
    bucket_sizes: &BTreeSet<NonZero<Price>>,
) -> StdResult<()> {
    let amount_quote = amount_base.checked_mul(price)?;

    for bucket_size in bucket_sizes {
        let bucket = get_bucket(**bucket_size, direction, price)?;
        let key = ((base_denom, quote_denom), **bucket_size, direction, bucket);

        DEPTHS.modify(storage, key, |depths| -> StdResult<_> {
            let (mut depth_base, mut depth_quote) = depths;

            depth_base.checked_sub_assign(amount_base)?;
            depth_quote.checked_sub_assign(amount_quote)?;

            // If the depth of either base or quote is zero, delete it from
            // storage.
            // It's possible one is zero while the other is non-zero, due to
            // rounding error.
            if depth_base.is_non_zero() || depth_quote.is_non_zero() {
                Ok(Some((depth_base, depth_quote)))
            } else {
                Ok(None)
            }
        })?;
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::constants::{
            FIFTY, ONE, ONE_HUNDRED, ONE_HUNDREDTH, ONE_TENTH, ONE_THOUSANDTH, TEN,
        },
        grug::Uint128,
        std::str::FromStr,
        test_case::test_case,
    };

    #[test_case(
        ONE_THOUSANDTH,
        Direction::Bid,
        Price::from_str("123.45").unwrap()
        => Price::from_str("123.450").unwrap()
    )]
    #[test_case(
        ONE_HUNDREDTH,
        Direction::Bid,
        Price::from_str("123.45").unwrap()
        => Price::from_str("123.45").unwrap()
    )]
    #[test_case(
        ONE_TENTH,
        Direction::Bid,
        Price::from_str("123.45").unwrap()
        => Price::from_str("123.4").unwrap()
    )]
    #[test_case(
        ONE,
        Direction::Bid,
        Price::from_str("123.45").unwrap()
        => Price::new(123)
    )]
    #[test_case(
        TEN,
        Direction::Bid,
        Price::from_str("123.45").unwrap()
        => Price::new(120)
    )]
    #[test_case(
        FIFTY,
        Direction::Bid,
        Price::from_str("123.45").unwrap()
        => Price::new(100)
    )]
    #[test_case(
        ONE_HUNDRED,
        Direction::Bid,
        Price::from_str("123.456").unwrap()
        => Price::new(100)
    )]
    #[test_case(
        ONE_THOUSANDTH,
        Direction::Ask,
        Price::from_str("123.45").unwrap()
        => Price::from_str("123.450").unwrap()
    )]
    #[test_case(
        ONE_HUNDREDTH,
        Direction::Ask,
        Price::from_str("123.45").unwrap()
        => Price::from_str("123.45").unwrap()
    )]
    #[test_case(
        ONE_TENTH,
        Direction::Ask,
        Price::from_str("123.45").unwrap()
        => Price::from_str("123.5").unwrap() // ceil
    )]
    #[test_case(
        ONE,
        Direction::Ask,
        Price::from_str("123.45").unwrap()
        => Price::new(124) // ceil
    )]
    #[test_case(
        TEN,
        Direction::Ask,
        Price::from_str("123.45").unwrap()
        => Price::new(130) // ceil
    )]
    #[test_case(
        FIFTY,
        Direction::Ask,
        Price::from_str("123.45").unwrap()
        => Price::new(150) // ceil
    )]
    #[test_case(
        ONE_HUNDRED,
        Direction::Ask,
        Price::from_str("123.45").unwrap()
        => Price::new(200) // ceil
    )]
    // This is an error case we found via proptesting.
    //
    // Prior to PR #1208, we compute `lower` as follows:
    //
    // ```rust
    // let lower = price
    //     .checked_div(bucket_size)?
    //     .checked_floor()?
    //     .checked_mul(bucket_size)?;
    // ```
    //
    // With the following input, it would panic at the `.checked_div` step.
    #[test_case(
        ONE_HUNDREDTH,
        Direction::Bid,
        Price::raw(Uint128::new(4368614282292957095945129368187275851))
        => Price::from_str("4368614282292.95").unwrap(); // floor
        "no overflow - bid"
    )]
    #[test_case(
        ONE_HUNDREDTH,
        Direction::Ask,
        Price::raw(Uint128::new(4368614282292957095945129368187275851))
        => Price::from_str("4368614282292.96").unwrap(); // ceil
        "no overflow - ask"
    )]
    fn getting_bucket(bucket_size: Price, direction: Direction, price: Price) -> Price {
        get_bucket(bucket_size, direction, price).unwrap()
    }
}
