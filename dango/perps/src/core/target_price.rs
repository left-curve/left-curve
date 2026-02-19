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

/// Returns whether the execution price violates the price constraint.
pub fn is_price_constraint_violated(exec_price: UsdPrice, target_price: UsdPrice, is_bid: bool) -> bool {
    if is_bid {
        exec_price > target_price
    } else {
        exec_price < target_price
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_types::perps::PairParam, test_case::test_case};

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
        assert_eq!(
            compute_target_price(
                OrderKind::Market {
                    max_slippage: Ratio::new_permille(slippage_permille),
                },
                UsdPrice::new_int(100),
                HumanAmount::new(skew),
                &PairParam::new_mock(100, 50),
                is_bid
            )
            .unwrap(),
            UsdPrice::new_raw(expected_raw)
        );
    }

    #[test_case(105, 0, true,  105_000_000 ; "limit bid ignores skew")]
    #[test_case( 95, 0, false,  95_000_000 ; "limit ask ignores skew")]
    #[test_case(110, 5, true,  110_000_000 ; "limit bid nonzero skew")]
    fn compute_target_price_limit_works(limit: i128, skew: i128, is_bid: bool, expected_raw: i128) {
        assert_eq!(
            compute_target_price(
                OrderKind::Limit {
                    limit_price: UsdPrice::new_int(limit),
                },
                UsdPrice::new_int(100),
                HumanAmount::new(skew),
                &PairParam::new_mock(100, 50),
                is_bid
            )
            .unwrap(),
            UsdPrice::new_raw(expected_raw)
        );
    }

    #[test_case( 99, 101, true,  false ; "bid exec below target")]
    #[test_case(101, 101, true,  false ; "bid exec equals target")]
    #[test_case(102, 101, true,  true  ; "bid exec above target")]
    #[test_case(100,  99, false, false ; "ask exec above target")]
    #[test_case( 99,  99, false, false ; "ask exec equals target")]
    #[test_case( 98,  99, false, true  ; "ask exec below target")]
    fn is_price_constraint_violated_works(exec: i128, target: i128, is_bid: bool, expected: bool) {
        assert_eq!(
            is_price_constraint_violated(UsdPrice::new_int(exec), UsdPrice::new_int(target), is_bid),
            expected
        );
    }
}
