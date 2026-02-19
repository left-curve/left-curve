use {
    crate::core::compute_marginal_price,
    dango_types::{
        HumanAmount, Ratio, UsdPrice,
        perps::{OrderKind, PairParam},
    },
    grug::MathResult,
};

/// Compute the target price of an order. The order must be executed at a price
/// better than or equal to this price.
pub fn compute_target_price(
    kind: OrderKind,
    oracle_price: UsdPrice,
    skew: HumanAmount,
    pair_param: &PairParam,
    is_bid: bool,
) -> MathResult<UsdPrice> {
    match kind {
        OrderKind::Market { max_slippage } => {
            let marginal_price = compute_marginal_price(oracle_price, skew, pair_param)?;
            if is_bid {
                marginal_price.checked_mul(Ratio::ONE.checked_add(max_slippage)?)
            } else {
                marginal_price.checked_mul(Ratio::ONE.checked_sub(max_slippage)?)
            }
        },
        OrderKind::Limit { limit_price } => Ok(limit_price),
    }
}

/// Returns whether the execution price is better than or equal to the target price.
pub fn check_price_constraint(exec_price: UsdPrice, target_price: UsdPrice, is_bid: bool) -> bool {
    if is_bid {
        exec_price <= target_price
    } else {
        exec_price >= target_price
    }
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
    #[test_case(  0,   0, true,  100_000_000 ; "zero slippage bid")]
    #[test_case(  0,   0, false, 100_000_000 ; "zero slippage ask")]
    #[test_case(  0,  10, true,  101_000_000 ; "neutral bid 1pct")]
    #[test_case(  0,  10, false,  99_000_000 ; "neutral ask 1pct")]
    #[test_case(  3,  10, true,  104_030_000 ; "positive skew bid 1pct")]
    #[test_case(  3,  10, false, 101_970_000 ; "positive skew ask 1pct")]
    #[test_case( -3,  10, true,   97_970_000 ; "negative skew bid 1pct")]
    #[test_case( -3,  10, false,  96_030_000 ; "negative skew ask 1pct")]
    #[test_case(  0,  50, true,  105_000_000 ; "neutral bid 5pct")]
    #[test_case(  0,  50, false,  95_000_000 ; "neutral ask 5pct")]
    fn compute_target_price_market_works(
        skew: i128,
        slippage_permille: i128,
        is_bid: bool,
        expected_raw: i128,
    ) {
        let kind = OrderKind::Market {
            max_slippage: slippage(slippage_permille),
        };
        assert_eq!(
            compute_target_price(kind, price(100), human(skew), &mock_pair_param(100, 50), is_bid)
                .unwrap(),
            price_raw(expected_raw)
        );
    }

    #[test_case(105, 0, true,  105_000_000 ; "limit bid ignores skew")]
    #[test_case( 95, 0, false,  95_000_000 ; "limit ask ignores skew")]
    #[test_case(110, 5, true,  110_000_000 ; "limit bid nonzero skew")]
    fn compute_target_price_limit_works(
        limit: i128,
        skew: i128,
        is_bid: bool,
        expected_raw: i128,
    ) {
        let kind = OrderKind::Limit {
            limit_price: price(limit),
        };
        assert_eq!(
            compute_target_price(kind, price(100), human(skew), &mock_pair_param(100, 50), is_bid)
                .unwrap(),
            price_raw(expected_raw)
        );
    }

    #[test_case( 99, 101, true,  true  ; "bid exec below target")]
    #[test_case(101, 101, true,  true  ; "bid exec equals target")]
    #[test_case(102, 101, true,  false ; "bid exec above target")]
    #[test_case(100,  99, false, true  ; "ask exec above target")]
    #[test_case( 99,  99, false, true  ; "ask exec equals target")]
    #[test_case( 98,  99, false, false ; "ask exec below target")]
    fn check_price_constraint_works(exec: i128, target: i128, is_bid: bool, expected: bool) {
        assert_eq!(
            check_price_constraint(price(exec), price(target), is_bid),
            expected
        );
    }

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

    fn slippage(permille: i128) -> Ratio<UsdPrice> {
        Ratio::new(Dec128_6::new_permille(permille))
    }
}
