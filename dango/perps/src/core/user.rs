use {
    crate::NoCachePerpQuerier,
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, UsdPrice, UsdValue,
        perps::{OrderKind, PairId, PairParam, PairState, Param, Position, UserState},
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
pub(super) fn compute_position_unrealized_funding(
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
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<UsdValue> {
    let mut total_pnl = UsdValue::ZERO;
    let mut total_funding = UsdValue::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_state = perp_querier.query_pair_state(pair_id)?;

        total_pnl.checked_add_assign(compute_position_unrealized_pnl(position, oracle_price)?)?;
        total_funding
            .checked_add_assign(compute_position_unrealized_funding(position, &pair_state)?)?;
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
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<UsdValue> {
    let mut total = UsdValue::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_param = perp_querier.query_pair_param(pair_id)?;

        let margin = position
            .size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.maintenance_margin_ratio)?;

        total.checked_add_assign(margin)?;
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
pub(super) fn compute_initial_margin(
    user_state: &UserState,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
    projected_pair_id: &PairId,
    projected_size: Quantity,
) -> anyhow::Result<UsdValue> {
    let mut total = UsdValue::ZERO;
    let mut projected_pair_seen = false;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_param = perp_querier.query_pair_param(pair_id)?;

        let size = if pair_id == projected_pair_id {
            projected_pair_seen = true;
            projected_size
        } else {
            position.size
        };

        let margin = size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.initial_margin_ratio)?;

        total.checked_add_assign(margin)?;
    }

    // If the projected pair is not in existing positions and the projected size
    // is non-zero, add its margin contribution.
    if !projected_pair_seen && projected_size.is_non_zero() {
        let oracle_price = oracle_querier.query_price_for_perps(projected_pair_id)?;
        let pair_param = perp_querier.query_pair_param(projected_pair_id)?;

        let margin = projected_size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.initial_margin_ratio)?;

        total.checked_add_assign(margin)?;
    }

    Ok(total)
}

/// Compute the margin required for the opening portion of a limit order.
///
/// ```plain
/// required = |opening_size| * limit_price * initial_margin_ratio
/// ```
///
/// Only the opening portion (new exposure) requires margin reservation.
/// Returns zero when `opening_size` is zero (pure closing order).
pub fn compute_required_margin(
    opening_size: Quantity,
    limit_price: UsdPrice,
    pair_param: &PairParam,
) -> grug::MathResult<UsdValue> {
    opening_size
        .checked_abs()?
        .checked_mul(limit_price)?
        .checked_mul(pair_param.initial_margin_ratio)
}

/// Compute the margin available for new orders or withdrawals.
///
/// ```plain
/// available = max(0, equity - used_margin - reserved_margin_value)
/// ```
///
/// where `used_margin = Σ |size| * oracle_price * initial_margin_ratio`
/// over all existing positions (no projection).
///
/// Returns zero when equity falls below the used + reserved requirement
/// (the user cannot open new positions or withdraw, and may face liquidation).
pub(super) fn compute_available_margin(
    collateral_value: UsdValue,
    user_state: &UserState,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
    reserved_margin_value: UsdValue,
) -> anyhow::Result<UsdValue> {
    let equity = compute_user_equity(collateral_value, user_state, perp_querier, oracle_querier)?;

    let mut used_margin = UsdValue::ZERO;
    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_param = perp_querier.query_pair_param(pair_id)?;

        let margin = position
            .size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.initial_margin_ratio)?;

        used_margin.checked_add_assign(margin)?;
    }

    Ok(equity
        .checked_sub(used_margin)?
        .checked_sub(reserved_margin_value)?
        .max(UsdValue::ZERO))
}

/// Returns true if the user is eligible for liquidation.
///
/// A user is liquidatable when their equity (collateral + unrealized PnL
/// - accrued funding) falls below their total maintenance margin.
///
/// A user with no open positions is never liquidatable.
pub fn is_liquidatable(
    collateral_value: UsdValue,
    user_state: &UserState,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<bool> {
    if user_state.positions.is_empty() {
        return Ok(false);
    }

    let equity = compute_user_equity(collateral_value, user_state, perp_querier, oracle_querier)?;
    let maintenance_margin = compute_maintenance_margin(user_state, perp_querier, oracle_querier)?;

    Ok(equity < maintenance_margin)
}

/// Ensure the user's collateral balance satisfies both extreme cases:
///
/// 1. The order is filled 100%: user's equity must be no less than required
///    initial margin + reserved margin for existing resting orders + fee.
/// 2. The order is filled 0%: user's available margin must be no less than
///    required margin for this order + fee. (This check is only performed for
///    GTC orders.)
///
/// In practice, the order may be filled anywhere between 0% to 100%. However,
/// if the user's margin satisfies both the extreme cases, it must satisfy all
/// general cases.
pub fn check_margin(
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
    taker_state: &UserState,
    param: &Param,
    pair_param: &PairParam,
    pair_id: &PairId,
    oracle_price: UsdPrice,
    collateral_value: UsdValue,
    size: Quantity,
    kind: OrderKind,
) -> anyhow::Result<()> {
    // -------------------------- Check 1: 100% fill ---------------------------

    let equity = compute_user_equity(collateral_value, taker_state, perp_querier, oracle_querier)?;

    let current_position = taker_state
        .positions
        .get(pair_id)
        .map(|p| p.size)
        .unwrap_or_default();

    let projected_size = current_position.checked_add(size)?;

    let projected_im = compute_initial_margin(
        taker_state,
        perp_querier,
        oracle_querier,
        pair_id,
        projected_size,
    )?;

    let projected_fee = super::compute_trading_fee(size, oracle_price, param.taker_fee_rate)?;

    let required_margin = projected_im
        .checked_add(projected_fee)?
        .checked_add(taker_state.reserved_margin)?;

    ensure!(
        equity >= required_margin,
        "insufficient margin: equity ({}) < initial margin ({}) + fee ({}) + reserved ({})",
        equity,
        projected_im,
        projected_fee,
        taker_state.reserved_margin
    );

    // --------------------------- Check 2: 0% fill ----------------------------

    if let OrderKind::Limit { limit_price } = kind {
        let available_margin = compute_available_margin(
            collateral_value,
            taker_state,
            perp_querier,
            oracle_querier,
            taker_state.reserved_margin,
        )?;

        let projected_rm = compute_required_margin(size, limit_price, pair_param)?;

        let projected_fee = super::compute_trading_fee(size, limit_price, param.taker_fee_rate)?;

        let required_margin = projected_rm.checked_add(projected_fee)?;

        ensure!(
            available_margin >= required_margin,
            "insufficient margin for limit order: available ({}) < required ({}) + fee ({})",
            available_margin,
            projected_rm,
            projected_fee
        );
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue,
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
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(entry_price),
            entry_funding_per_unit: FundingPerUnit::new_int(0),
        };

        assert_eq!(
            compute_position_unrealized_pnl(&position, UsdPrice::new_int(oracle_price)).unwrap(),
            UsdValue::new_int(expected),
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
            size: Quantity::new_raw(size_raw),
            entry_price: UsdPrice::new_raw(0),
            entry_funding_per_unit: FundingPerUnit::new_raw(entry_raw),
        };
        let pair_state = PairState {
            funding_per_unit: FundingPerUnit::new_raw(cumulative_raw),
            ..Default::default()
        };

        assert_eq!(
            compute_position_unrealized_funding(&position, &pair_state).unwrap(),
            UsdValue::new_raw(expected_raw),
        );
    }

    // ---- compute_user_equity tests ----

    #[test]
    fn equity_no_positions() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_user_equity(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new_int(10_000),
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
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            HashMap::new(),
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                }
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_user_equity(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new_int(15_000),
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
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(1),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            HashMap::new(),
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(3),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_user_equity(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new_int(14_980),
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
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(1),
                },
                btc::DENOM.clone() => Position {
                    size: Quantity::new_int(-1),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            HashMap::new(),
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(3),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
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
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new_int(16_980),
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
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            HashMap::new(),
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(150_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_user_equity(
                UsdValue::new_int(100),
                &user_state,
                &perp_querier,
                &mut oracle_querier
            )
            .unwrap(),
            UsdValue::new_int(-4900),
        );
    }

    // ---- compute_maintenance_margin tests ----

    #[test]
    fn maintenance_margin_no_positions() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_maintenance_margin(&user_state, &perp_querier, &mut oracle_querier).unwrap(),
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
                    size: Quantity::new_int(size),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Dimensionless::new_permille(50),
                    ..Default::default()
                },
            },
            HashMap::new(),
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_maintenance_margin(&user_state, &perp_querier, &mut oracle_querier).unwrap(),
            UsdValue::new_int(1000),
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
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
                btc::DENOM.clone() => Position {
                    size: Quantity::new_int(-1),
                    entry_price: UsdPrice::new_int(50000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Dimensionless::new_permille(50),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Dimensionless::new_permille(30),
                    ..Default::default()
                },
            },
            HashMap::new(),
            None,
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
            compute_maintenance_margin(&user_state, &perp_querier, &mut oracle_querier).unwrap(),
            UsdValue::new_int(2500),
        );
    }

    // ---- compute_initial_margin tests ----

    // No existing positions; project 10 ETH @ $2000, 10% IMR
    // margin = |10| * 2000 * 0.10 = $2000
    #[test]
    fn initial_margin_no_existing_positions() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            HashMap::new(),
            None,
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
                &perp_querier,
                &mut oracle_querier,
                &eth::DENOM,
                Quantity::new_int(10),
            )
            .unwrap(),
            UsdValue::new_int(2000),
        );
    }

    // Has 5 ETH position; project to 10 ETH → uses projected size (10)
    // margin = |10| * 2000 * 0.10 = $2000
    #[test]
    fn initial_margin_projects_existing_position() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(5),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            HashMap::new(),
            None,
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
                &perp_querier,
                &mut oracle_querier,
                &eth::DENOM,
                Quantity::new_int(10),
            )
            .unwrap(),
            UsdValue::new_int(2000),
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
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            HashMap::new(),
            None,
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
                &perp_querier,
                &mut oracle_querier,
                &btc::DENOM,
                Quantity::new_int(1),
            )
            .unwrap(),
            UsdValue::new_int(7000),
        );
    }

    // No positions; project 0 size → $0 (new pair with zero size not added)
    #[test]
    fn initial_margin_zero_projected_size_skipped() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            HashMap::new(),
            None,
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
                &perp_querier,
                &mut oracle_querier,
                &eth::DENOM,
                Quantity::ZERO,
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
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
                btc::DENOM.clone() => Position {
                    size: Quantity::new_int(-1),
                    entry_price: UsdPrice::new_int(50000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(50),
                    ..Default::default()
                },
            },
            HashMap::new(),
            None,
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
                &perp_querier,
                &mut oracle_querier,
                &eth::DENOM,
                Quantity::new_int(20),
            )
            .unwrap(),
            UsdValue::new_int(6500),
        );
    }

    // ---- compute_required_margin tests ----

    // required = |opening_size| * limit_price * initial_margin_ratio
    #[test_case( 0,  2000, 100,    0 ; "zero opening size")]
    #[test_case( 10, 2000, 100, 2000 ; "long opening")]
    #[test_case(-10, 2000, 100, 2000 ; "short opening")]
    #[test_case( 1, 50000,  50, 2500 ; "high price low imr")]
    fn compute_required_margin_works(
        opening_size: i128,
        limit_price: i128,
        imr_permille: i128,
        expected: i128,
    ) {
        let pair_param = PairParam {
            initial_margin_ratio: Dimensionless::new_permille(imr_permille),
            ..Default::default()
        };

        assert_eq!(
            compute_required_margin(
                Quantity::new_int(opening_size),
                UsdPrice::new_int(limit_price),
                &pair_param,
            )
            .unwrap(),
            UsdValue::new_int(expected),
        );
    }

    // ---- compute_available_margin tests ----

    // collateral=10000, no positions, no reserved → available = 10000
    #[test]
    fn available_margin_no_positions() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_available_margin(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
                UsdValue::ZERO,
            )
            .unwrap(),
            UsdValue::new_int(10_000),
        );
    }

    // collateral=10000, ETH long 10 @ entry=2000, oracle=2500
    // pnl = 10 * (2500 - 2000) = 5000, no funding, no reserved
    // equity = 15000, used = |10| * 2500 * 0.10 = 2500
    // available = 15000 - 2500 = 12500
    #[test]
    fn available_margin_with_profit() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_available_margin(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
                UsdValue::ZERO,
            )
            .unwrap(),
            UsdValue::new_int(12_500),
        );
    }

    // Same as above but reserved=2000
    // available = 12500 - 2000 = 10500
    #[test]
    fn available_margin_with_reserved() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_available_margin(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
                UsdValue::new_int(2_000),
            )
            .unwrap(),
            UsdValue::new_int(10_500),
        );
    }

    // collateral=100, ETH long 10 @ entry=2000, oracle=1500
    // pnl = 10 * (1500 - 2000) = -5000
    // equity = 100 - 5000 = -4900
    // used = |10| * 1500 * 0.10 = 1500
    // raw = -4900 - 1500 = -6400, clamped to 0
    #[test]
    fn available_margin_clamped_to_zero() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(150_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_available_margin(
                UsdValue::new_int(100),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
                UsdValue::ZERO,
            )
            .unwrap(),
            UsdValue::ZERO,
        );
    }

    // collateral=10000, ETH long 10 @ entry=2000, oracle=2500
    // funding: funding_per_unit=3, entry=1, delta=2, accrued=10*2=20
    // equity = 10000 + 5000 - 20 = 14980
    // used = |10| * 2500 * 0.10 = 2500, no reserved
    // available = 14980 - 2500 = 12480
    #[test]
    fn available_margin_with_funding() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(1),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(100),
                    ..Default::default()
                },
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(3),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_available_margin(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
                UsdValue::ZERO,
            )
            .unwrap(),
            UsdValue::new_int(12_480),
        );
    }

    // ---- is_liquidatable tests ----

    #[test]
    fn is_liquidatable_no_positions() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert!(
            !is_liquidatable(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
            )
            .unwrap()
        );
    }

    // collateral=10000, ETH long 10 @ entry=2000, oracle=2500, mmr=5%
    // equity = 10000 + 10*(2500-2000) = 15000
    // maint  = |10| * 2500 * 0.05 = 1250
    // 15000 >= 1250 → not liquidatable
    #[test]
    fn is_liquidatable_healthy() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Dimensionless::new_permille(50),
                    ..Default::default()
                },
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert!(
            !is_liquidatable(
                UsdValue::new_int(10_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
            )
            .unwrap()
        );
    }

    // equity exactly equals maintenance margin → not liquidatable (strict <)
    // Need: equity = maint. maint = |10| * 2000 * 0.05 = 1000
    // equity = collateral + pnl - funding = collateral + 0 - 0 = collateral
    // So collateral = 1000
    #[test]
    fn is_liquidatable_at_boundary() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Dimensionless::new_permille(50),
                    ..Default::default()
                },
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert!(
            !is_liquidatable(
                UsdValue::new_int(1_000),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
            )
            .unwrap()
        );
    }

    // collateral=100, ETH long 10 @ entry=2000, oracle=1500, mmr=5%
    // equity = 100 + 10*(1500-2000) = 100 - 5000 = -4900
    // maint  = |10| * 1500 * 0.05 = 750
    // -4900 < 750 → liquidatable
    #[test]
    fn is_liquidatable_underwater() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam {
                    maintenance_margin_ratio: Dimensionless::new_permille(50),
                    ..Default::default()
                },
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(150_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert!(
            is_liquidatable(
                UsdValue::new_int(100),
                &user_state,
                &perp_querier,
                &mut oracle_querier,
            )
            .unwrap()
        );
    }

    // Funding can push a user into liquidation territory.
    //
    // Setup: ETH long 10 @ entry=2000, oracle=2000 (no pnl), mmr=5%
    //   funding_per_unit=100, entry=0 → accrued = 10 * 100 = 1000
    //   maint = |10| * 2000 * 0.05 = 1000
    //
    // Case 1: collateral=10000
    //   equity = 10000 + 0 - 1000 = 9000, maint=1000 → not liquidatable
    //
    // Case 2: collateral=900
    //   equity = 900 + 0 - 1000 = -100, maint=1000 → liquidatable
    #[test]
    fn is_liquidatable_funding_pushes_under() {
        let make_fixtures = |collateral: i128| {
            let user_state = UserState {
                positions: btree_map! {
                    eth::DENOM.clone() => Position {
                        size: Quantity::new_int(10),
                        entry_price: UsdPrice::new_int(2000),
                        entry_funding_per_unit: FundingPerUnit::new_int(0),
                    },
                },
                ..Default::default()
            };
            let perp_querier = NoCachePerpQuerier::new_mock(
                hash_map! {
                    eth::DENOM.clone() => PairParam {
                        maintenance_margin_ratio: Dimensionless::new_permille(50),
                        ..Default::default()
                    },
                },
                hash_map! {
                    eth::DENOM.clone() => PairState {
                        funding_per_unit: FundingPerUnit::new_int(100),
                        ..Default::default()
                    },
                },
                None,
            );
            let oracle_querier = OracleQuerier::new_mock(hash_map! {
                eth::DENOM.clone() => PrecisionedPrice::new(
                    Udec128::new_percent(200_000),
                    Timestamp::from_seconds(0),
                    18,
                ),
            });
            (
                UsdValue::new_int(collateral),
                user_state,
                perp_querier,
                oracle_querier,
            )
        };

        // Case 1: healthy despite funding
        let (col, us, pq, mut oq) = make_fixtures(10_000);
        assert!(!is_liquidatable(col, &us, &pq, &mut oq).unwrap());

        // Case 2: funding pushes equity below maintenance margin
        let (col, us, pq, mut oq) = make_fixtures(900);
        assert!(is_liquidatable(col, &us, &pq, &mut oq).unwrap());
    }

    // ---- check_margin tests ----

    /// 100%-fill check fails: equity ($25,200) < IM ($25,000) + fee ($500) = $25,500
    ///
    /// No existing position. oracle = $50,000, IMR = 5%, taker_fee = 0.1%
    /// Buy limit 10 BTC @ $49,000
    ///
    /// 100% fill:
    ///   projected_im = |10| * 50,000 * 0.05 = $25,000
    ///   fee          = |10| * 50,000 * 0.001 = $500
    ///   Need equity >= $25,500, but equity = $25,200 → FAILS
    ///
    /// 0% fill:
    ///   margin_to_reserve = |10| * 49,000 * 0.05 + |10| * 49,000 * 0.001
    ///                     = $24,500 + $490 = $24,990
    ///   available = equity = $25,200
    ///   $25,200 >= $24,990 → passes
    #[test]
    fn margin_check_full_fill_fails() {
        let pair_id: PairId = "perp/btcusd".parse().unwrap();
        let param = Param {
            taker_fee_rate: Dimensionless::new_permille(1), // 0.1%
            ..Default::default()
        };
        let pair_param = PairParam {
            initial_margin_ratio: Dimensionless::new_permille(50), // 5%
            ..Default::default()
        };
        let taker_state = UserState::default();

        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                pair_id.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(50),
                    ..Default::default()
                },
            },
            hash_map! {
                pair_id.clone() => PairState::default(),
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = check_margin(
            &perp_querier,
            &mut oracle_querier,
            &taker_state,
            &param,
            &pair_param,
            &pair_id,
            UsdPrice::new_int(50_000),
            UsdValue::new_int(25_200),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(49_000),
            },
        );

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("insufficient margin:"),
            "expected 100%-fill margin error, got: {msg}"
        );
    }

    /// 0%-fill check fails: available ($27,000) < reserved ($27,500) + fee ($550) = $28,050
    ///
    /// No existing position. oracle = $50,000, IMR = 5%, taker_fee = 0.1%
    /// Buy limit 10 BTC @ $55,000
    ///
    /// 100% fill:
    ///   projected_im = |10| * 50,000 * 0.05 = $25,000
    ///   fee          = |10| * 50,000 * 0.001 = $500
    ///   Need equity >= $25,500, equity = $27,000 → passes
    ///
    /// 0% fill:
    ///   margin_to_reserve = |10| * 55,000 * 0.05 + |10| * 55,000 * 0.001
    ///                     = $27,500 + $550 = $28,050
    ///   available = equity = $27,000
    ///   $27,000 < $28,050 → FAILS
    #[test]
    fn margin_check_zero_fill_fails() {
        let pair_id: PairId = "perp/btcusd".parse().unwrap();
        let param = Param {
            taker_fee_rate: Dimensionless::new_permille(1), // 0.1%
            ..Default::default()
        };
        let pair_param = PairParam {
            initial_margin_ratio: Dimensionless::new_permille(50), // 5%
            ..Default::default()
        };
        let taker_state = UserState::default();

        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                pair_id.clone() => PairParam {
                    initial_margin_ratio: Dimensionless::new_permille(50),
                    ..Default::default()
                },
            },
            hash_map! {
                pair_id.clone() => PairState::default(),
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = check_margin(
            &perp_querier,
            &mut oracle_querier,
            &taker_state,
            &param,
            &pair_param,
            &pair_id,
            UsdPrice::new_int(50_000),
            UsdValue::new_int(27_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(55_000),
            },
        );

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("insufficient margin for limit order"),
            "expected 0%-fill margin error, got: {msg}"
        );
    }
}
