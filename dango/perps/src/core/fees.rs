use {
    dango_types::{
        BaseAmount, HumanAmount, UsdPrice,
        perps::{Param, settlement_currency},
    },
    grug::MathResult,
};

/// Given the fillable size of an order and other relevant information, compute
/// the amount of trading fee (denoted in the base units of the settlement currency).
pub fn compute_trading_fee(
    fill_size: HumanAmount,
    oracle_price: UsdPrice,
    settlement_currency_price: UsdPrice,
    param: &Param,
) -> MathResult<BaseAmount> {
    fill_size
        .checked_abs()?
        .checked_mul(oracle_price)?
        .checked_mul(param.trading_fee_rate)?
        .checked_div(settlement_currency_price)?
        .checked_into_base_ceil(settlement_currency::DECIMAL)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_types::Ratio, test_case::test_case};

    // (fill_size, oracle_price, settlement_price, fee_rate_raw, expected_base)
    #[test_case(   0,      1,     1,   1_000,          0 ; "zero fill")]
    #[test_case( 100,      1,     1,   1_000,    100_000 ; "simple 0.1 percent fee")]
    #[test_case(-100,      1,     1,   1_000,    100_000 ; "negative fill same result")]
    #[test_case(   1, 50_000,     1,     500, 25_000_000 ; "high oracle price")]
    fn compute_trading_fee_works(
        fill_size: i128,
        oracle_price: i128,
        settlement_price: i128,
        fee_rate_raw: i128,
        expected_base: u128,
    ) {
        assert_eq!(
            compute_trading_fee(
                HumanAmount::new(fill_size),
                UsdPrice::new_int(oracle_price),
                UsdPrice::new_int(settlement_price),
                &Param {
                    trading_fee_rate: Ratio::new_raw(fee_rate_raw),
                    ..Default::default()
                },
            )
            .unwrap(),
            BaseAmount::new(expected_base),
        );
    }
}
