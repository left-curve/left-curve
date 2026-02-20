use {
    crate::NoCachePairQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        HumanAmount, UsdValue,
        perps::{PairId, UserState},
    },
};

/// Compute the margin required to maintain the user's open positions, in USD.
///
/// For each position, the maintenance margin is:
///
/// ```plain
/// |position.size| * oracle_price * maintenance_margin_ratio
/// ```
///
/// The total maintenance margin is the sum of that of all positions.
///
/// The maintenance margin acts as the liquidation trigger. If a user's collateral
/// value falls below the maintenance margin, he becomes eligible for liquidation.
pub fn compute_maintenance_margin(
    user_state: &UserState,
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<UsdValue> {
    let mut total = UsdValue::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_param = pair_querier.query_pair_param(pair_id)?;

        let margin = position
            .size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.maintenance_margin_ratio)?;

        total = total.checked_add(margin)?;
    }

    Ok(total)
}

/// Compute the margin required to open a new position, in USD.
///
/// For each position, the initial margin is:
///
/// ```plain
/// |position.size| * oracle_price * initial_margin_ratio
/// ```
///
/// The total initial margin is the sum of that of all positions.
/// One position's size is overriden by a "projected" value, reflecting the size
/// if the order is executed.
///
/// When submitting an order, the user must have no less collateral than the
/// initial margin, otherwise the order is rejected.
pub fn compute_initial_margin(
    user_state: &UserState,
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
    projected_pair_id: PairId,
    projected_size: HumanAmount,
) -> anyhow::Result<UsdValue> {
    let mut total = UsdValue::ZERO;
    let mut projected_pair_seen = false;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_param = pair_querier.query_pair_param(pair_id)?;

        let size = if *pair_id == projected_pair_id {
            projected_pair_seen = true;
            projected_size
        } else {
            position.size
        };

        let margin = size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.initial_margin_ratio)?;

        total = total.checked_add(margin)?;
    }

    // If the projected pair is not in existing positions and the projected size
    // is non-zero, add its margin contribution.
    if !projected_pair_seen && projected_size.is_non_zero() {
        let oracle_price = oracle_querier.query_price_for_perps(&projected_pair_id)?;
        let pair_param = pair_querier.query_pair_param(&projected_pair_id)?;

        let margin = projected_size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.initial_margin_ratio)?;

        total = total.checked_add(margin)?;
    }

    Ok(total)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            HumanAmount, Ratio, UsdPrice,
            constants::{btc, eth},
            oracle::PrecisionedPrice,
            perps::{PairParam, Position},
        },
        grug::{Timestamp, Udec128, btree_map, hash_map},
        std::collections::HashMap,
        test_case::test_case,
    };

    #[test]
    fn maintenance_margin_no_positions() {
        let user_state = UserState::default();
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_maintenance_margin(&user_state, &pair_querier, &mut oracle_querier).unwrap(),
            UsdValue::ZERO,
        );
    }

    // size = 10 or -10, oracle = $2000, mmr = 5%
    // margin = |size| * 2000 * 0.05 = $1000
    #[test_case( 10 ; "long position")]
    #[test_case(-10 ; "short position")]
    fn maintenance_margin_single_position_works(size: i128) {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(size),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                maintenance_margin_ratio: Ratio::new_permille(50),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_maintenance_margin(&user_state, &pair_querier, &mut oracle_querier).unwrap(),
            UsdValue::new(1000),
        );
    }

    // ETH: |10| * $2000 * 5% = $1000
    // BTC: |1|  * $50000 * 3% = $1500
    // Total = $2500
    #[test]
    fn maintenance_margin_multiple_positions() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
                btc::DENOM.clone() => Position {
                    size: HumanAmount::new(-1),
                    entry_price: UsdPrice::new_int(50000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                maintenance_margin_ratio: Ratio::new_permille(50),
                ..Default::default()
            },
            btc::DENOM.clone() => PairParam {
                maintenance_margin_ratio: Ratio::new_permille(30),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
            btc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        assert_eq!(
            compute_maintenance_margin(&user_state, &pair_querier, &mut oracle_querier).unwrap(),
            UsdValue::new(2500),
        );
    }

    // ---- compute_initial_margin tests ----

    // No existing positions; project 10 ETH @ $2000, 10% IMR
    // margin = |10| * 2000 * 0.10 = $2000
    #[test]
    fn initial_margin_no_existing_positions() {
        let user_state = UserState::default();
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                initial_margin_ratio: Ratio::new_permille(100),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_initial_margin(
                &user_state,
                &pair_querier,
                &mut oracle_querier,
                eth::DENOM.clone(),
                HumanAmount::new(10),
            )
            .unwrap(),
            UsdValue::new(2000),
        );
    }

    // Has 5 ETH position; project to 10 ETH → uses projected size (10)
    // margin = |10| * 2000 * 0.10 = $2000
    #[test]
    fn initial_margin_projects_existing_position() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(5),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                initial_margin_ratio: Ratio::new_permille(100),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_initial_margin(
                &user_state,
                &pair_querier,
                &mut oracle_querier,
                eth::DENOM.clone(),
                HumanAmount::new(10),
            )
            .unwrap(),
            UsdValue::new(2000),
        );
    }

    // Has ETH position; project BTC (not in positions)
    // ETH: |10| * 2000 * 0.10 = $2000
    // BTC: |1|  * 50000 * 0.10 = $5000
    // Total = $7000
    #[test]
    fn initial_margin_adds_new_pair() {
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
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                initial_margin_ratio: Ratio::new_permille(100),
                ..Default::default()
            },
            btc::DENOM.clone() => PairParam {
                initial_margin_ratio: Ratio::new_permille(100),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
            btc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        assert_eq!(
            compute_initial_margin(
                &user_state,
                &pair_querier,
                &mut oracle_querier,
                btc::DENOM.clone(),
                HumanAmount::new(1),
            )
            .unwrap(),
            UsdValue::new(7000),
        );
    }

    // No positions; project 0 size → $0 (new pair with zero size not added)
    #[test]
    fn initial_margin_zero_projected_size_skipped() {
        let user_state = UserState::default();
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                initial_margin_ratio: Ratio::new_permille(100),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_initial_margin(
                &user_state,
                &pair_querier,
                &mut oracle_querier,
                eth::DENOM.clone(),
                HumanAmount::ZERO,
            )
            .unwrap(),
            UsdValue::ZERO,
        );
    }

    // Has ETH + BTC; project ETH to 20
    // ETH: |20| * 2000 * 0.10 = $4000 (projected)
    // BTC: |1|  * 50000 * 0.05 = $2500 (existing)
    // Total = $6500
    #[test]
    fn initial_margin_mixed() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: HumanAmount::new(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
                btc::DENOM.clone() => Position {
                    size: HumanAmount::new(-1),
                    entry_price: UsdPrice::new_int(50000),
                    entry_funding_per_unit: Ratio::new_int(0),
                },
            },
            ..Default::default()
        };
        let pair_querier = NoCachePairQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PairParam {
                initial_margin_ratio: Ratio::new_permille(100),
                ..Default::default()
            },
            btc::DENOM.clone() => PairParam {
                initial_margin_ratio: Ratio::new_permille(50),
                ..Default::default()
            },
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
            btc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        assert_eq!(
            compute_initial_margin(
                &user_state,
                &pair_querier,
                &mut oracle_querier,
                eth::DENOM.clone(),
                HumanAmount::new(20),
            )
            .unwrap(),
            UsdValue::new(6500),
        );
    }
}
