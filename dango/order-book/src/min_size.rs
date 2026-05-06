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
}
