use {
    super::compute_accrued_funding,
    crate::NoCachePairQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        UsdPrice, UsdValue,
        perps::{Position, UserState},
    },
};

/// Compute the unrealized PnL of a single position at the given oracle price.
///
/// ```plain
/// pnl = size * (oracle_price - entry_price)
/// ```
///
/// Positive result means profit; negative means loss. The sign automatically
/// accounts for position direction (long vs short).
fn compute_position_unrealized_pnl(
    position: &Position,
    oracle_price: UsdPrice,
) -> grug::MathResult<UsdValue> {
    let price_delta = oracle_price.checked_sub(position.entry_price)?;
    position.size.checked_mul(price_delta)
}

/// Compute a user's equity (net account value) across all open positions.
///
/// ```plain
/// equity = collateral_value + Σ(unrealized_pnl) - Σ(accrued_funding)
/// ```
pub fn compute_user_equity(
    collateral_value: UsdValue,
    user_state: &UserState,
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<UsdValue> {
    let mut total_pnl = UsdValue::ZERO;
    let mut total_funding = UsdValue::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_state = pair_querier.query_pair_state(pair_id)?;

        total_pnl =
            total_pnl.checked_add(compute_position_unrealized_pnl(position, oracle_price)?)?;
        total_funding =
            total_funding.checked_add(compute_accrued_funding(position, &pair_state)?)?;
    }

    Ok(collateral_value
        .checked_add(total_pnl)?
        .checked_sub(total_funding)?)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            HumanAmount, Ratio, UsdPrice, UsdValue,
            constants::{btc, eth},
            oracle::PrecisionedPrice,
            perps::{PairState, Position},
        },
        grug::{Timestamp, Udec128, btree_map, hash_map},
        std::collections::HashMap,
        test_case::test_case,
    };

    // ---- compute_position_unrealized_pnl tests ----

    // pnl = size * (oracle_price - entry_price)
    #[test_case( 10, 2000, 2500,  5000 ; "long in profit")]
    #[test_case( 10, 2000, 1500, -5000 ; "long in loss")]
    #[test_case(-10, 2000, 1500,  5000 ; "short in profit")]
    #[test_case(-10, 2000, 2500, -5000 ; "short in loss")]
    #[test_case( 10, 2000, 2000,     0 ; "no price change")]
    fn compute_position_unrealized_pnl_works(
        size: i128,
        entry_price: i128,
        oracle_price: i128,
        expected: i128,
    ) {
        let position = Position {
            size: HumanAmount::new(size),
            entry_price: UsdPrice::new_int(entry_price),
            entry_funding_per_unit: Ratio::new_int(0),
        };

        assert_eq!(
            compute_position_unrealized_pnl(&position, UsdPrice::new_int(oracle_price)).unwrap(),
            UsdValue::new(expected),
        );
    }

    // ---- compute_user_equity tests ----

    #[test]
    fn equity_no_positions() {
        let user_state = UserState::default();
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_user_equity(
                UsdValue::new(10_000),
                &user_state,
                &pair_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new(10_000),
        );
    }

    // collateral=10000, ETH long 10 @ entry=2000, oracle=2500
    // pnl = 10 * (2500 - 2000) = 5000, no funding
    // equity = 10000 + 5000 = 15000
    #[test]
    fn equity_single_position_pnl_only() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), hash_map! {
            eth::DENOM.clone() => PairState {
                funding_per_unit: Ratio::new_int(0),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_user_equity(
                UsdValue::new(10_000),
                &user_state,
                &pair_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new(15_000),
        );
    }

    // Same position but with funding:
    //   funding_per_unit=3, entry_funding_per_unit=1, delta=2
    //   accrued_funding = 10 * 2 = 20
    //   equity = 10000 + 5000 - 20 = 14980
    #[test]
    fn equity_with_pnl_and_funding() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(1),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), hash_map! {
            eth::DENOM.clone() => PairState {
                funding_per_unit: Ratio::new_int(3),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_user_equity(
                UsdValue::new(10_000),
                &user_state,
                &pair_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new(14_980),
        );
    }

    // Two positions:
    //   ETH long 10 @ entry=2000, oracle=2500, funding delta=2
    //     pnl = 5000, funding = 20
    //   BTC short -1 @ entry=50000, oracle=48000, no funding
    //     pnl = -1 * (48000 - 50000) = 2000, funding = 0
    //   equity = 10000 + (5000 + 2000) - (20 + 0) = 16980
    #[test]
    fn equity_multiple_positions() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(1),
                },
                btc::DENOM.clone() => Position {
                    size: HumanAmount::new(-1),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), hash_map! {
            eth::DENOM.clone() => PairState {
                funding_per_unit: Ratio::new_int(3),
                ..Default::default()
            },
            btc::DENOM.clone() => PairState {
                funding_per_unit: Ratio::new_int(0),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
            btc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(4_800_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        assert_eq!(
            compute_user_equity(
                UsdValue::new(10_000),
                &user_state,
                &pair_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new(16_980),
        );
    }

    // collateral=100, ETH long 10 @ entry=2000, oracle=1500
    // pnl = 10 * (1500 - 2000) = -5000
    // equity = 100 + (-5000) - 0 = -4900
    #[test]
    fn equity_negative() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), hash_map! {
            eth::DENOM.clone() => PairState {
                funding_per_unit: Ratio::new_int(0),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(150_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_user_equity(
                UsdValue::new(100),
                &user_state,
                &pair_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new(-4900),
        );
    }
}
