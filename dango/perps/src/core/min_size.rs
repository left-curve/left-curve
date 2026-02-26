use {
    anyhow::ensure,
    dango_types::{Quantity, UsdPrice, perps::PairParam},
};

/// Ensure the total order notional is no smaller than the pair's minimum order
/// size. Reduce-only orders are exempt (the caller skips this check).
pub fn check_minimum_order_size(
    size: Quantity,
    oracle_price: UsdPrice,
    pair_param: &PairParam,
) -> anyhow::Result<()> {
    let notional = size.checked_abs()?.checked_mul(oracle_price)?;
    ensure!(
        notional >= pair_param.min_order_size,
        "order size is below minimum: {} < {}",
        notional,
        pair_param.min_order_size
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_types::UsdValue, test_case::test_case};

    // (size, oracle_price, min_order_size, should_pass)
    #[test_case( 1, 100, 100, true  ; "notional exactly at minimum")]
    #[test_case( 2, 100, 100, true  ; "notional above minimum")]
    #[test_case( 1, 100, 200, false ; "notional below minimum")]
    #[test_case(-1, 100, 100, true  ; "negative size exactly at minimum")]
    #[test_case(-2, 100, 100, true  ; "negative size above minimum")]
    #[test_case(-1, 100, 200, false ; "negative size below minimum")]
    fn check_minimum_order_size_works(
        size: i128,
        oracle_price: i128,
        min_order_size: i128,
        should_pass: bool,
    ) {
        assert_eq!(
            check_minimum_order_size(
                Quantity::new_int(size),
                UsdPrice::new_int(oracle_price),
                &PairParam {
                    min_order_size: UsdValue::new_int(min_order_size),
                    ..Default::default()
                }
            )
            .is_ok(),
            should_pass
        );
    }
}
