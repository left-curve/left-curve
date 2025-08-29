use {
    crate::DEPTHS,
    dango_types::dex::Direction,
    grug::{
        Decimal, Denom, IsZero, MathResult, NonZero, Number, StdResult, Storage, Udec128_6,
        Udec128_24,
    },
    std::collections::BTreeSet,
};

/// For bids, return the bucket that is immediately smaller than the price.
/// For asks, return the bucket that is immediately larger than the price.
pub fn get_bucket(
    bucket_size: Udec128_24,
    direction: Direction,
    price: Udec128_24,
) -> MathResult<Udec128_24> {
    // This is the bucket immediately smaller than the price.
    let lower = price
        .checked_div(bucket_size)?
        .checked_floor()?
        .checked_mul(bucket_size)?;

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
    price: Udec128_24,
    amount_base: Udec128_6,
    amount_quote: Udec128_6,
    bucket_sizes: &BTreeSet<NonZero<Udec128_24>>,
) -> StdResult<()> {
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
/// This is called under three circumstances:
/// - in `execute::batch_update_orders`, when canceling user limit orders;
/// - in `cron::auction`, when canceling passive orders from the previous block;
/// - in `cron::clear_orders_of_pair`, when a limit order (user or passive) is fulfilled.
pub fn decrease_liquidity_depths(
    storage: &mut dyn Storage,
    base_denom: &Denom,
    quote_denom: &Denom,
    direction: Direction,
    price: Udec128_24,
    amount_base: Udec128_6,
    amount_quote: Udec128_6,
    bucket_sizes: &BTreeSet<NonZero<Udec128_24>>,
) -> StdResult<()> {
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
