use {
    crate::NoCachePairQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        HumanAmount, UsdPrice, UsdValue,
        perps::{PairId, PairState, Position, UserState},
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
    let delta = oracle_price.checked_sub(position.entry_price)?;
    position.size.checked_mul(delta)
}

/// Compute the funding accrued by a specific position since it was
/// last touched (opened, modified, or had funding settled).
///
/// accrued = position.size * (current_cumulative - entry_cumulative)
///
/// Sign convention:
///
/// - Positive result = trader owes vault (cost to the trader)
/// - Negative result = vault owes trader (credit to the trader)
///
/// This follows from:
///
/// - When rate > 0: longs pay (size > 0 produces positive accrued)
/// - When rate < 0: shorts pay (size < 0, delta < 0, product is positive)
pub fn compute_position_unrealized_funding(
    position: &Position,
    pair_state: &PairState,
) -> grug::MathResult<UsdValue> {
    let delta = (pair_state.funding_per_unit).checked_sub(position.entry_funding_per_unit)?;
    position.size.checked_mul(delta)
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
        total_funding = total_funding
            .checked_add(compute_position_unrealized_funding(position, &pair_state)?)?;
    }

    Ok(collateral_value
        .checked_add(total_pnl)?
        .checked_sub(total_funding)?)
}

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
            HumanAmount, Ratio, UsdPrice, UsdValue,
            constants::{btc, eth},
            oracle::PrecisionedPrice,
            perps::{PairParam, PairState, Position},
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

    // ---- compute_position_unrealized_funding tests ----

    // accrued = size * (cumulative - entry_funding_per_unit)
    //
    // Raw math example ("long pays"):
    //   cumulative = 7_500_000 raw (7.5), entry = 5_000_000 raw (5.0)
    //   delta = 2_500_000 raw (2.5)
    //   size = 10_000_000 raw (10)
    //   accrued = (10_000_000 * 2_500_000) / 1_000_000 = 25_000_000 raw (25 USD)
    #[test_case( 10_000_000, 5_000_000, 5_000_000,          0 ; "no delta")]
    #[test_case( 10_000_000, 7_500_000, 5_000_000,  25_000_000 ; "long pays")]
    #[test_case(-10_000_000, 7_500_000, 5_000_000, -25_000_000 ; "short receives")]
    #[test_case( 10_000_000, 3_000_000, 5_000_000, -20_000_000 ; "long receives")]
    #[test_case(-10_000_000, 3_000_000, 5_000_000,  20_000_000 ; "short pays")]
    fn compute_position_unrealized_funding_works(
        size_raw: i128,
        cumulative_raw: i128,
        entry_raw: i128,
        expected_raw: i128,
    ) {
        let position = Position {
            size: HumanAmount::new(size_raw),
            entry_price: Ratio::new_raw(0),
            entry_funding_per_unit: Ratio::new_raw(entry_raw),
        };
        let pair_state = PairState {
            funding_per_unit: Ratio::new_raw(cumulative_raw),
            ..Default::default()
        };

        assert_eq!(
            compute_position_unrealized_funding(&position, &pair_state).unwrap(),
            UsdValue::new(expected_raw),
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

    // ---- compute_maintenance_margin tests ----

    #[test]
    fn maintenance_margin_no_positions() {
        let user_state = UserState::default();
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new());
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
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Ratio::new_permille(50),
                    ..Default::default()
                },
            },
            HashMap::new(),
        );
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
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Ratio::new_permille(50),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Ratio::new_permille(30),
                    ..Default::default()
                },
            },
            HashMap::new(),
        );
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
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Ratio::new_permille(100),
                    ..Default::default()
                },
            },
            HashMap::new(),
        );
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
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Ratio::new_permille(100),
                    ..Default::default()
                },
            },
            HashMap::new(),
        );
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
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Ratio::new_permille(100),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairParam {
                    initial_margin_ratio: Ratio::new_permille(100),
                    ..Default::default()
                },
            },
            HashMap::new(),
        );
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
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Ratio::new_permille(100),
                    ..Default::default()
                },
            },
            HashMap::new(),
        );
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
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Ratio::new_permille(100),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairParam {
                    initial_margin_ratio: Ratio::new_permille(50),
                    ..Default::default()
                },
            },
            HashMap::new(),
        );
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
