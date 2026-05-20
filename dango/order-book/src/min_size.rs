use {
    crate::{Quantity, UsdPrice, UsdValue},
    anyhow::ensure,
};

/// Ensure the total order notional is no smaller than the pair's minimum
/// order value. Reduce-only orders are exempt (the caller skips this check).
pub fn check_minimum_order_value(
    size: Quantity,
    oracle_price: UsdPrice,
    min_order_value: UsdValue,
) -> anyhow::Result<()> {
    let notional = size.checked_abs()?.checked_mul(oracle_price)?;

    ensure!(
        notional >= min_order_value,
        "order value is below minimum: {} < {}",
        notional,
        min_order_value
    );

    Ok(())
}

/// Ensure that an order's `size` is a non-zero integer multiple of the
/// pair's `lot_size` — i.e., `|size| >= lot_size` and `|size| % lot_size == 0`.
///
/// This is the precision-floor half of the protocol's dust-prevention design
/// (the value floor is `min_order_value`); enforcing it at submission means
/// every fill, partial fill, and resulting position is automatically a
/// non-negative multiple of `lot_size` by induction.
///
/// The lower-bound (`|size| >= lot_size`) is the same shape Hyperliquid /
/// Binance / dYdX use: one lot is the minimum tradable size; sub-lot orders
/// are rejected. The modulo check (`|size| % lot_size == 0`) makes the
/// position-size space discrete.
///
/// `lot_size = 0` disables the check — every `size` is treated as valid.
/// Used during initial chain bring-up before per-pair lot sizes are tuned.
pub fn check_lot_size(size: Quantity, lot_size: Quantity) -> anyhow::Result<()> {
    if lot_size.is_zero() {
        return Ok(());
    }

    let abs_size = size.checked_abs()?;

    ensure!(
        abs_size >= lot_size,
        "order size is below lot size: |{}| < {}",
        size,
        lot_size,
    );

    ensure!(
        abs_size.checked_rem(lot_size)?.is_zero(),
        "order size is not a multiple of lot size: |{}| % {} != 0",
        size,
        lot_size,
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, test_case::test_case};

    // (size, oracle_price, min_order_value, should_pass)
    #[test_case( 1, 100, 100, true  ; "notional exactly at minimum")]
    #[test_case( 2, 100, 100, true  ; "notional above minimum")]
    #[test_case( 1, 100, 200, false ; "notional below minimum")]
    #[test_case(-1, 100, 100, true  ; "negative size exactly at minimum")]
    #[test_case(-2, 100, 100, true  ; "negative size above minimum")]
    #[test_case(-1, 100, 200, false ; "negative size below minimum")]
    fn check_minimum_order_value_works(
        size: i128,
        oracle_price: i128,
        min_order_value: i128,
        should_pass: bool,
    ) {
        assert_eq!(
            check_minimum_order_value(
                Quantity::new_int(size),
                UsdPrice::new_int(oracle_price),
                UsdValue::new_int(min_order_value),
            )
            .is_ok(),
            should_pass
        );
    }

    // (size, lot_size, should_pass)
    //
    // Two failure modes for a positive `lot_size`:
    //   - `|size| < lot_size`     → below the minimum tradable size
    //   - `|size| % lot_size != 0` → not lot-aligned
    //
    // Sign of `size` is irrelevant to alignment (longs and shorts are both
    // checked the same way). `lot_size = 0` disables the check — any
    // `size` is accepted, including zero and sub-lot values.
    #[test_case(   0, 5, false ; "zero size below lot size")]
    #[test_case(   2, 5, false ; "positive below one lot")]
    #[test_case(  -2, 5, false ; "negative below one lot")]
    #[test_case(   5, 5, true  ; "exactly one lot")]
    #[test_case(  -5, 5, true  ; "exactly one lot negative")]
    #[test_case(  10, 5, true  ; "two lots positive")]
    #[test_case( -10, 5, true  ; "two lots negative")]
    #[test_case(   7, 5, false ; "between one and two lots")]
    #[test_case(  -7, 5, false ; "between one and two lots negative")]
    #[test_case(   1, 1, true  ; "lot of one accepts any non-zero")]
    #[test_case(   0, 0, true  ; "lot zero disables: zero accepted")]
    #[test_case(   1, 0, true  ; "lot zero disables: sub-lot accepted")]
    #[test_case(  -7, 0, true  ; "lot zero disables: negative accepted")]
    fn check_lot_size_works(size: i128, lot_size: i128, should_pass: bool) {
        assert_eq!(
            check_lot_size(Quantity::new_int(size), Quantity::new_int(lot_size)).is_ok(),
            should_pass,
        );
    }
}
