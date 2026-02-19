use {
    dango_types::{HumanAmount, Ratio, UsdPrice, perps::PairParam},
    grug::MathResult,
};

/// Compute the execution price of an order, given the current oracle price,
/// current skew, and the order's size.
///
/// ```plain
/// premium = clamp(
///   (skew + size / 2) / skew_scale,
///   -max_abs_premium,
///   max_abs_premium
/// )
///
/// exec_price = oracle_price * (1 + premium)
/// ```
pub fn compute_exec_price(
    oracle_price: UsdPrice,
    skew: HumanAmount,
    size: HumanAmount,
    pair_param: &PairParam,
) -> MathResult<UsdPrice> {
    // The average between the current skew and the skew after this order has
    // been executed in full.
    let skew_average = skew.checked_add(size.checked_mul(Ratio::HALF)?)?;

    // Compute a premium based on the average skew and skew scaling factor.
    let premium = skew_average.checked_div(pair_param.skew_scale)?;

    // Bound the premium between [-max_abs_premium, max_abs_premium].
    let premium = premium.clamp(-pair_param.max_abs_premium, pair_param.max_abs_premium);

    oracle_price.checked_mul(premium.checked_add(Ratio::ONE)?)
}

/// Compute the marginal price, given the current oracle price and skew.
///
/// Marginal price is the price for executing an order of infinitesimal size.
pub fn compute_marginal_price(
    oracle_price: UsdPrice,
    skew: HumanAmount,
    pair_param: &PairParam,
) -> MathResult<UsdPrice> {
    compute_exec_price(oracle_price, skew, HumanAmount::ZERO, pair_param)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{FromInner, UsdValue},
        grug::{Dec128_6, Int128, NumberConst},
        test_case::test_case,
    };

    // oracle_price = 100, skew_scale = 100, max_abs_premium = 0.05
    #[test_case(  0,     0,  100_000_000 ; "neutral market no order")]
    #[test_case(  0,    10,  105_000_000 ; "neutral market buy pushes to max")]
    #[test_case(  0,   -10,   95_000_000 ; "neutral market sell pushes to neg max")]
    #[test_case(  0,     4,  102_000_000 ; "neutral market small buy unclamped")]
    #[test_case(  0,    -4,   98_000_000 ; "neutral market small sell unclamped")]
    #[test_case(  5,     5,  105_000_000 ; "positive skew buy clamped at max")]
    #[test_case(  5,    -5,  102_500_000 ; "positive skew sell partially cancels")]
    #[test_case( -5,     5,   97_500_000 ; "negative skew buy partially cancels")]
    #[test_case( -5,    -5,   95_000_000 ; "negative skew sell clamped at neg max")]
    #[test_case(  5,   -10,  100_000_000 ; "positive skew sell exactly cancels")]
    #[test_case( -5,    10,  100_000_000 ; "negative skew buy exactly cancels")]
    #[test_case(100,   100,  105_000_000 ; "extreme positive hard clamp")]
    #[test_case(-100, -100,   95_000_000 ; "extreme negative hard clamp")]
    fn compute_exec_price_works(skew: i128, size: i128, expected_raw: i128) {
        assert_eq!(
            compute_exec_price(
                price(100),
                human(skew),
                human(size),
                &mock_pair_param(100, 50)
            )
            .unwrap(),
            price_raw(expected_raw)
        );
    }

    // oracle_price = 100, skew_scale = 100, max_abs_premium = 0.05
    #[test_case(  0,  100_000_000 ; "neutral")]
    #[test_case(  3,  103_000_000 ; "positive skew")]
    #[test_case( -3,   97_000_000 ; "negative skew")]
    #[test_case( 50,  105_000_000 ; "clamped at positive max")]
    #[test_case(-50,   95_000_000 ; "clamped at negative max")]
    fn compute_marginal_price_works(skew: i128, expected_raw: i128) {
        assert_eq!(
            compute_marginal_price(price(100), human(skew), &mock_pair_param(100, 50)).unwrap(),
            price_raw(expected_raw)
        );
    }

    /// Build a `PairParam` with the two pricing-relevant fields varied;
    /// all other fields use inert dummy values.
    fn mock_pair_param(skew_scale: i128, max_abs_premium_permille: i128) -> PairParam {
        PairParam {
            skew_scale: Ratio::new(Dec128_6::new(skew_scale)),
            max_abs_premium: Ratio::new(Dec128_6::new_permille(max_abs_premium_permille)),
            max_abs_oi: HumanAmount::from_inner(Dec128_6::new(1_000_000)),
            max_abs_funding_rate: Ratio::new(Dec128_6::ZERO),
            max_funding_velocity: Ratio::new(Dec128_6::ZERO),
            min_opening_size: UsdValue::from_inner(Dec128_6::ZERO),
            initial_margin_ratio: Ratio::new(Dec128_6::ZERO),
            maintenance_margin_ratio: Ratio::new(Dec128_6::ZERO),
        }
    }

    fn human(n: i128) -> HumanAmount {
        HumanAmount::from_inner(Dec128_6::new(n))
    }

    fn price(n: i128) -> UsdPrice {
        Ratio::new(Dec128_6::new(n))
    }

    fn price_raw(raw: i128) -> UsdPrice {
        Ratio::new(Dec128_6::raw(Int128::new(raw)))
    }
}
