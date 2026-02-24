use {
    dango_types::{Quantity, UsdPrice, UsdValue, perps::Param},
    grug::MathResult,
};

/// Given the fillable size of an order and other relevant information, compute
/// the trading fee as a USD value.
pub fn compute_trading_fee(
    fill_size: Quantity,
    exec_price: UsdPrice,
    param: &Param,
) -> MathResult<UsdValue> {
    fill_size
        .checked_abs()?
        .checked_mul(exec_price)?
        .checked_mul(param.trading_fee_rate)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_types::Dimensionless, test_case::test_case};

    // (fill_size, exec_price, fee_rate_raw, expected_raw)
    #[test_case(   0,      1,   1_000,          0 ; "zero fill")]
    #[test_case( 100,      1,   1_000,    100_000 ; "simple 0.1 percent fee")]
    #[test_case(-100,      1,   1_000,    100_000 ; "negative fill same result")]
    #[test_case(   1, 50_000,     500, 25_000_000 ; "high exec price")]
    fn compute_trading_fee_works(
        fill_size: i128,
        exec_price: i128,
        fee_rate_raw: i128,
        expected_raw: i128,
    ) {
        assert_eq!(
            compute_trading_fee(
                Quantity::new_int(fill_size),
                UsdPrice::new_int(exec_price),
                &Param {
                    trading_fee_rate: Dimensionless::new_raw(fee_rate_raw),
                    ..Default::default()
                },
            )
            .unwrap(),
            UsdValue::new_raw(expected_raw),
        );
    }
}
