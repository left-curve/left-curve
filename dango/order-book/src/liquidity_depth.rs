use {
    crate::{PairId, Quantity, UsdPrice, state::DEPTHS},
    grug::{MathResult, StdResult, Storage},
    std::collections::BTreeSet,
};

/// Compute the bucket boundary for a given price.
///
/// - **Bid** (`is_bid = true`): floor to the nearest bucket ≤ price.
/// - **Ask** (`is_bid = false`): ceil to the nearest bucket ≥ price.
fn get_bucket(bucket_size: UsdPrice, is_bid: bool, price: UsdPrice) -> MathResult<UsdPrice> {
    if is_bid {
        price.checked_floor_multiple(bucket_size)
    } else {
        price.checked_ceil_multiple(bucket_size)
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
    use {
        super::*,
        grug::{Dec128_6, MockStorage},
        std::str::FromStr,
        test_case::test_case,
    };

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

    /// Notional drift from Dec128_6 truncation in `size * price`.
    ///
    /// Dec128_6 multiplication truncates: `trunc(a * p)` drops fractional
    /// digits beyond 6 decimal places. This means the notional computed for
    /// a whole order differs from the sum of notionals computed for its parts:
    ///
    /// ```text
    /// price        = 3.000001
    /// initial_size = 1.333333
    /// filled_size  = 0.666666
    /// remaining    = 1.333333 - 0.666666 = 0.666667
    ///
    /// notional(initial)   = trunc(1.333333 * 3.000001) = trunc(4.000000333...) = 4.000000
    /// notional(filled)    = trunc(0.666666 * 3.000001) = trunc(1.999998666...) = 1.999998
    /// notional(remaining) = trunc(0.666667 * 3.000001) = trunc(2.000001666...) = 2.000001
    /// ```
    ///
    /// **Naive approach**: subtract `notional(filled)` from the stored depth:
    ///
    /// ```text
    /// stored = notional(initial) - notional(filled)
    ///        = 4.000000 - 1.999998
    ///        = 2.000002   <-- should be 2.000001, off by 0.000001
    /// ```
    ///
    /// The depth now reports a notional that no longer equals `remaining * price`.
    /// Over many partial fills this drift accumulates.
    ///
    /// **Correct approach**: remove all depth, re-add with `remaining`:
    ///
    /// ```text
    /// stored = 0 + notional(remaining)
    ///        = 2.000001   <-- exact
    /// ```
    ///
    /// By recomputing the notional from scratch each time, the stored value
    /// always equals `remaining * price` with no accumulated error.
    #[test]
    fn partial_fill_no_residual_depth() {
        let mut storage = MockStorage::new();
        let pair_id: PairId = "perp/ethusd".parse().unwrap();
        let bucket_sizes = BTreeSet::from([ONE]);
        let price = p("3.000001");
        let is_bid = true;

        let initial_size = Quantity::new(Dec128_6::from_str("1.333333").unwrap());
        let filled_size = Quantity::new(Dec128_6::from_str("0.666666").unwrap());
        let remaining = initial_size.checked_sub(filled_size).unwrap();
        let remaining_notional = remaining.checked_mul(price).unwrap();

        let bucket = get_bucket(ONE, is_bid, price).unwrap();
        let key = (&pair_id, ONE, is_bid, bucket);

        // Part A: naive subtract-filled produces notional drift.

        {
            increase_liquidity_depths(
                &mut storage,
                &pair_id,
                is_bid,
                price,
                initial_size,
                &bucket_sizes,
            )
            .unwrap();

            // Subtract just the filled portion.
            decrease_liquidity_depths(
                &mut storage,
                &pair_id,
                is_bid,
                price,
                filled_size,
                &bucket_sizes,
            )
            .unwrap();

            // Stored notional = notional(initial) - notional(filled) = 2.000002,
            // but remaining * price = 2.000001. Off by one unit.
            let (size, notional) = DEPTHS.load(&storage, key).unwrap();

            assert_eq!(size, remaining, "size should equal remaining");
            assert_ne!(
                notional, remaining_notional,
                "naive approach should produce notional drift"
            );

            // Clean up for Part B.
            decrease_liquidity_depths(
                &mut storage,
                &pair_id,
                is_bid,
                price,
                remaining,
                &bucket_sizes,
            )
            .unwrap();
        }

        // Part B: remove-all + re-add (current pattern) is exact.

        {
            increase_liquidity_depths(
                &mut storage,
                &pair_id,
                is_bid,
                price,
                initial_size,
                &bucket_sizes,
            )
            .unwrap();

            // Remove all old depth, then re-add with remaining size.
            decrease_liquidity_depths(
                &mut storage,
                &pair_id,
                is_bid,
                price,
                initial_size,
                &bucket_sizes,
            )
            .unwrap();

            increase_liquidity_depths(
                &mut storage,
                &pair_id,
                is_bid,
                price,
                remaining,
                &bucket_sizes,
            )
            .unwrap();

            // Notional matches `remaining * price` exactly.
            let (size, notional) = DEPTHS.load(&storage, key).unwrap();

            assert_eq!(size, remaining, "size should equal remaining");
            assert_eq!(
                notional, remaining_notional,
                "notional should equal remaining * price exactly"
            );

            // Final cleanup — depth drops to zero.
            decrease_liquidity_depths(
                &mut storage,
                &pair_id,
                is_bid,
                price,
                remaining,
                &bucket_sizes,
            )
            .unwrap();

            assert!(
                !DEPTHS.has(&storage, key),
                "depth should be fully cleaned up"
            );
        }
    }
}
