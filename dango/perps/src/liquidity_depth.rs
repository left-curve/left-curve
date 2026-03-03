use {
    crate::DEPTHS,
    dango_types::{Quantity, UsdPrice, perps::PairId},
    grug::{Dec128_6, Fraction, MathResult, Number, StdResult, Storage},
    std::collections::BTreeSet,
};

/// Compute the bucket boundary for a given price.
///
/// - **Bid** (`is_bid = true`): floor to the nearest bucket ≤ price.
/// - **Ask** (`is_bid = false`): ceil to the nearest bucket ≥ price.
pub fn get_bucket(bucket_size: UsdPrice, is_bid: bool, price: UsdPrice) -> MathResult<UsdPrice> {
    // Work with the integer numerators of the inner Dec128_6 to minimise
    // expensive mul/div operations and avoid intermediate overflow.
    let price_num = price.into_inner().numerator();
    let bucket_num = bucket_size.into_inner().numerator();

    let lower_num = price_num.checked_div(bucket_num)?.checked_mul(bucket_num)?;
    let lower = UsdPrice::new(Dec128_6::raw(lower_num));

    debug_assert!(
        lower <= price,
        "lower bucket ({lower}) is somehow bigger than the price ({price})"
    );

    // If the order is an ask and the price isn't exactly on a bucket boundary,
    // return one bucket higher.
    if !is_bid && price > lower {
        lower.checked_add(bucket_size)
    } else {
        Ok(lower)
    }
}

/// Increase the aggregated liquidity depth across all configured bucket sizes.
///
/// Called when a new resting order is placed on the book.
pub fn increase_liquidity_depths(
    storage: &mut dyn Storage,
    pair_id: &PairId,
    is_bid: bool,
    price: UsdPrice,
    abs_size: Quantity,
    bucket_sizes: &BTreeSet<UsdPrice>,
) -> StdResult<()> {
    let notional = abs_size.checked_mul(price)?;

    for bucket_size in bucket_sizes {
        let bucket = get_bucket(*bucket_size, is_bid, price)?;
        let key = (pair_id, *bucket_size, is_bid, bucket);

        DEPTHS.may_update(storage, key, |maybe_depths| -> StdResult<_> {
            let (mut depth_size, mut depth_notional) = maybe_depths.unwrap_or_default();

            depth_size.checked_add_assign(abs_size)?;
            depth_notional.checked_add_assign(notional)?;

            Ok((depth_size, depth_notional))
        })?;
    }

    Ok(())
}

/// Decrease the aggregated liquidity depth across all configured bucket sizes.
///
/// Called when a resting order is cancelled or filled.
pub fn decrease_liquidity_depths(
    storage: &mut dyn Storage,
    pair_id: &PairId,
    is_bid: bool,
    price: UsdPrice,
    abs_size: Quantity,
    bucket_sizes: &BTreeSet<UsdPrice>,
) -> StdResult<()> {
    let notional = abs_size.checked_mul(price)?;

    for bucket_size in bucket_sizes {
        let bucket = get_bucket(*bucket_size, is_bid, price)?;
        let key = (pair_id, *bucket_size, is_bid, bucket);

        DEPTHS.modify(storage, key, |depths| -> StdResult<_> {
            let (mut depth_size, mut depth_notional) = depths;

            depth_size.checked_sub_assign(abs_size)?;
            depth_notional.checked_sub_assign(notional)?;

            if depth_size.is_non_zero() {
                Ok(Some((depth_size, depth_notional)))
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
    use {super::*, std::str::FromStr, test_case::test_case};

    /// Helper to parse a decimal string into `UsdPrice`.
    fn p(s: &str) -> UsdPrice {
        UsdPrice::new(Dec128_6::from_str(s).unwrap())
    }

    const ONE_THOUSANDTH: UsdPrice = UsdPrice::new_permille(1);
    const ONE_HUNDREDTH: UsdPrice = UsdPrice::new_permille(10);
    const ONE_TENTH: UsdPrice = UsdPrice::new_permille(100);
    const ONE: UsdPrice = UsdPrice::new_int(1);
    const TEN: UsdPrice = UsdPrice::new_int(10);
    const FIFTY: UsdPrice = UsdPrice::new_int(50);
    const ONE_HUNDRED: UsdPrice = UsdPrice::new_int(100);

    #[test_case(ONE_THOUSANDTH, true,  p("123.45")  => p("123.450"))]
    #[test_case(ONE_HUNDREDTH,  true,  p("123.45")  => p("123.45"))]
    #[test_case(ONE_TENTH,      true,  p("123.45")  => p("123.4"))]
    #[test_case(ONE,            true,  p("123.45")  => UsdPrice::new_int(123))]
    #[test_case(TEN,            true,  p("123.45")  => UsdPrice::new_int(120))]
    #[test_case(FIFTY,          true,  p("123.45")  => UsdPrice::new_int(100))]
    #[test_case(ONE_HUNDRED,    true,  p("123.456") => UsdPrice::new_int(100))]
    #[test_case(ONE_THOUSANDTH, false, p("123.45")  => p("123.450"))]
    #[test_case(ONE_HUNDREDTH,  false, p("123.45")  => p("123.45"))]
    #[test_case(ONE_TENTH,      false, p("123.45")  => p("123.5"))]
    #[test_case(ONE,            false, p("123.45")  => UsdPrice::new_int(124))]
    #[test_case(TEN,            false, p("123.45")  => UsdPrice::new_int(130))]
    #[test_case(FIFTY,          false, p("123.45")  => UsdPrice::new_int(150))]
    #[test_case(ONE_HUNDRED,    false, p("123.45")  => UsdPrice::new_int(200))]
    fn getting_bucket(bucket_size: UsdPrice, is_bid: bool, price: UsdPrice) -> UsdPrice {
        get_bucket(bucket_size, is_bid, price).unwrap()
    }
}
