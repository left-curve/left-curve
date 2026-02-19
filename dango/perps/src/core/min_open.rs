use {
    anyhow::ensure,
    dango_types::{HumanAmount, UsdPrice, perps::PairParam},
};

/// If the order's opening portion is non-zero, ensure it's no smaller than the
/// minimum order size, such that we don't open dust positions.
pub fn check_minimum_opening(
    opening_size: HumanAmount,
    oracle_price: UsdPrice,
    pair_params: &PairParam,
) -> anyhow::Result<()> {
    if opening_size.is_non_zero() {
        let opening_notional = opening_size.checked_abs()?.checked_mul(oracle_price)?;
        ensure!(
            opening_notional >= pair_params.min_opening_size,
            "opening size is below minimum: {} < {}",
            opening_notional,
            pair_params.min_opening_size
        );
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_types::UsdValue, test_case::test_case};

    // (opening_size, oracle_price, min_opening_size, should_pass)
    #[test_case( 0, 100,  10, true  ; "zero opening always passes")]
    #[test_case( 1, 100, 100, true  ; "notional exactly at minimum")]
    #[test_case( 2, 100, 100, true  ; "notional above minimum")]
    #[test_case( 1, 100, 200, false ; "notional below minimum")]
    #[test_case(-1, 100, 100, true  ; "negative size exactly at minimum")]
    #[test_case(-2, 100, 100, true  ; "negative size above minimum")]
    #[test_case(-1, 100, 200, false ; "negative size below minimum")]
    fn check_minimum_opening_works(
        opening_size: i128,
        oracle_price: i128,
        min_opening_size: i128,
        should_pass: bool,
    ) {
        assert_eq!(
            check_minimum_opening(
                HumanAmount::new(opening_size),
                UsdPrice::new_int(oracle_price),
                &PairParam {
                    min_opening_size: UsdValue::new(min_opening_size),
                    ..Default::default()
                }
            )
            .is_ok(),
            should_pass
        );
    }
}
