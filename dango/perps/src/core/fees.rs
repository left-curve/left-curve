use {
    dango_order_book::{Dimensionless, Quantity, UsdPrice, UsdValue},
    grug::MathResult,
};

/// Compute the USD notional value of a fill.
pub fn compute_notional(fill_size: Quantity, exec_price: UsdPrice) -> MathResult<UsdValue> {
    fill_size.checked_abs()?.checked_mul(exec_price)
}

/// Given the fillable size of an order, the execution price, and the applicable
/// fee rate (maker or taker), compute the trading fee as a USD value.
pub fn compute_trading_fee(
    fill_size: Quantity,
    exec_price: UsdPrice,
    fee_rate: Dimensionless,
) -> MathResult<UsdValue> {
    compute_notional(fill_size, exec_price)?.checked_mul(fee_rate)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, test_case::test_case};

    // (fill_size, exec_price, fee_rate_raw, expected_raw)
    #[test_case(   0,      1,   1_000,          0 ; "zero fill")]
    #[test_case( 100,      1,   1_000,    100_000 ; "simple 0.1 percent fee")]
    #[test_case(-100,      1,   1_000,    100_000 ; "negative fill same result")]
    #[test_case(   1, 50_000,     500, 25_000_000 ; "high exec price")]
    #[test_case( 100,      1,  -1_000,   -100_000 ; "negative fee rate produces negative fee")]
    #[test_case(-100,      1,  -1_000,   -100_000 ; "negative fill and negative rate")]
    #[test_case(   1, 50_000,    -100, -5_000_000 ; "negative 1 bps on high price")]
    #[test_case( 100,      1,       0,          0 ; "zero fee rate")]
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
                Dimensionless::new_raw(fee_rate_raw),
            )
            .unwrap(),
            UsdValue::new_raw(expected_raw),
        );
    }
}
