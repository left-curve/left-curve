use {
    dango_order_book::{Dimensionless, UsdPrice},
    dango_types::perps::OrderKind,
    grug::MathResult,
};

/// Compute the target price of an order. The order must be executed at a price
/// better than or equal to this price.
///
/// For market orders, slippage is relative to the oracle price:
///
/// ```plain
/// bid_target_price = oracle_price * (1 + max_slippage)
/// ask_target_price = oracle_price * (1 - max_slippage)
/// ```
///
/// For limit orders, the target price is simply the limit price.
pub fn compute_target_price(
    kind: OrderKind,
    oracle_price: UsdPrice,
    is_bid: bool,
) -> MathResult<UsdPrice> {
    match kind {
        OrderKind::Market { max_slippage } => {
            if is_bid {
                oracle_price.checked_mul(Dimensionless::ONE.checked_add(max_slippage)?)
            } else {
                oracle_price.checked_mul(Dimensionless::ONE.checked_sub(max_slippage)?)
            }
        },
        OrderKind::Limit { limit_price, .. } => Ok(limit_price),
    }
}

/// Returns whether the execution price violates the price constraint.
pub fn is_price_constraint_violated(
    exec_price: UsdPrice,
    target_price: UsdPrice,
    is_bid: bool,
) -> bool {
    if is_bid {
        exec_price > target_price
    } else {
        exec_price < target_price
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, dango_types::perps::TimeInForce, test_case::test_case};

    // oracle_price = 100
    #[test_case(  0, true,  100_000_000 ; "zero slippage bid")]
    #[test_case(  0, false, 100_000_000 ; "zero slippage ask")]
    #[test_case( 10, true,  101_000_000 ; "bid 1pct slippage")]
    #[test_case( 10, false,  99_000_000 ; "ask 1pct slippage")]
    #[test_case( 50, true,  105_000_000 ; "bid 5pct slippage")]
    #[test_case( 50, false,  95_000_000 ; "ask 5pct slippage")]
    fn compute_target_price_market_works(
        slippage_permille: i128,
        is_bid: bool,
        expected_raw: i128,
    ) {
        assert_eq!(
            compute_target_price(
                OrderKind::Market {
                    max_slippage: Dimensionless::new_permille(slippage_permille),
                },
                UsdPrice::new_int(100),
                is_bid
            )
            .unwrap(),
            UsdPrice::new_raw(expected_raw)
        );
    }

    #[test_case(105, true,  105_000_000 ; "limit bid")]
    #[test_case( 95, false,  95_000_000 ; "limit ask")]
    #[test_case(110, true,  110_000_000 ; "limit bid higher")]
    fn compute_target_price_limit_works(limit: i128, is_bid: bool, expected_raw: i128) {
        assert_eq!(
            compute_target_price(
                OrderKind::Limit {
                    limit_price: UsdPrice::new_int(limit),
                    time_in_force: TimeInForce::GoodTilCanceled,
                    client_order_id: None,
                },
                UsdPrice::new_int(100),
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
            is_price_constraint_violated(
                UsdPrice::new_int(exec),
                UsdPrice::new_int(target),
                is_bid
            ),
            expected
        );
    }
}
