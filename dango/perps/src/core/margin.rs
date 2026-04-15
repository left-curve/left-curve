use {
    crate::{
        core::{compute_trading_fee, execute_fill},
        querier::NoCachePerpQuerier,
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        perps::{PairId, PairParam, PairState, Position, UserState},
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
pub fn compute_position_unrealized_pnl(
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
/// equity = user_state.margin + Σ(unrealized_pnl) - Σ(accrued_funding)
/// ```
pub fn compute_user_equity(
    oracle_querier: &mut OracleQuerier,
    perp_querier: &NoCachePerpQuerier,
    user_state: &UserState,
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

    Ok(user_state
        .margin
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
    oracle_querier: &mut OracleQuerier,
    perp_querier: &NoCachePerpQuerier,
    user_state: &UserState,
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

/// Compute the initial margin required across all of a user's open positions.
///
/// ```plain
/// IM = Σ |position.size| * oracle_price * initial_margin_ratio
/// ```
pub(super) fn compute_initial_margin(
    oracle_querier: &mut OracleQuerier,
    perp_querier: &NoCachePerpQuerier,
    user_state: &UserState,
) -> anyhow::Result<UsdValue> {
    let mut total = UsdValue::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_param = perp_querier.query_pair_param(pair_id)?;

        let margin = position
            .size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.initial_margin_ratio)?;

        total.checked_add_assign(margin)?;
    }

    Ok(total)
}

/// Compute the total notional value of all open positions.
///
/// ```plain
/// total_notional = Σ |position.size| * oracle_price
/// ```
///
/// Used by the withdrawal leverage cap to measure total exposure without the
/// IMR multiplier.
pub fn compute_total_notional(
    oracle_querier: &mut OracleQuerier,
    _perp_querier: &NoCachePerpQuerier,
    user_state: &UserState,
) -> anyhow::Result<UsdValue> {
    let mut total = UsdValue::ZERO;

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;

        let notional = position.size.checked_abs()?.checked_mul(oracle_price)?;

        total.checked_add_assign(notional)?;
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
/// available = max(0, equity - used_margin - reserved_margin)
/// ```
///
/// where `used_margin = Σ |size| * oracle_price * initial_margin_ratio`
/// over all existing positions (no projection).
///
/// Returns zero when equity falls below the used + reserved requirement
/// (the user cannot open new positions or withdraw, and may face liquidation).
pub fn compute_available_margin(
    oracle_querier: &mut OracleQuerier,
    perp_querier: &NoCachePerpQuerier,
    user_state: &UserState,
) -> anyhow::Result<UsdValue> {
    let equity = compute_user_equity(oracle_querier, perp_querier, user_state)?;

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
        .checked_sub(user_state.reserved_margin)?
        .max(UsdValue::ZERO))
}

/// Ensure the user can afford the worst-case 100%-fill scenario.
///
/// Simulates the fill at `target_price` on a throwaway clone:
///
/// 1. Run `execute_fill` at `target_price` to realize PnL and update positions.
/// 2. Settle PnL and fees into margin.
/// 3. Verify post-fill equity ≥ post-fill initial margin + reserved margin.
///
/// This catches both bad-price closes (catastrophic realized PnL) and
/// bad-price opens (immediate unrealized loss from entry far from oracle).
///
/// The 0%-fill scenario (limit-order reservation) is checked separately
/// inside `store_limit_order`.
pub fn check_margin(
    oracle_querier: &mut OracleQuerier,
    pair_id: &PairId,
    perp_querier: &NoCachePerpQuerier,
    pair_state: &PairState,
    taker_state: &UserState,
    taker_fee_rate: Dimensionless,
    target_price: UsdPrice,
    closing_size: Quantity,
    opening_size: Quantity,
) -> anyhow::Result<()> {
    // Simulate the fill at worst-case price on throwaway clones.
    let mut projected = taker_state.clone();
    let mut pair_state = pair_state.clone();

    let pnl = execute_fill(
        pair_id,
        &mut pair_state,
        &mut projected,
        target_price,
        closing_size,
        opening_size,
    )?;

    // Settle PnL and fee into margin (mirrors settle_pnls).
    projected.margin.checked_add_assign(pnl)?;

    let fillable_size = closing_size.checked_add(opening_size)?;
    let fee = compute_trading_fee(fillable_size, target_price, taker_fee_rate)?;
    projected.margin.checked_sub_assign(fee)?;

    // Check post-fill health.
    let equity = compute_user_equity(oracle_querier, perp_querier, &projected)?;

    let im = compute_initial_margin(oracle_querier, perp_querier, &projected)?;

    let required = im.checked_add(projected.reserved_margin)?;

    ensure!(
        equity >= required,
        "insufficient margin: projected equity ({equity}) < initial margin ({im}) + reserved ({})",
        projected.reserved_margin,
    );

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
            perps::{PairParam, PairState, Param, Position, RateSchedule},
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
            conditional_order_above: None,
            conditional_order_below: None,
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
            conditional_order_above: None,
            conditional_order_below: None,
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
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_user_equity(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(10_000),
        );
    }

    // collateral=10000, ETH long 10 @ entry=2000, oracle=2500
    // pnl = 10 * (2500 - 2000) = 5000, no funding
    // equity = 10000 + 5000 = 15000
    #[test]
    fn equity_single_position_pnl_only() {
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), hash_map! {
            eth::DENOM.clone() => PairState {
                funding_per_unit: FundingPerUnit::new_int(0),
                ..Default::default()
            }
        });
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_user_equity(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
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
            margin: UsdValue::new_int(10_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(1),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), hash_map! {
            eth::DENOM.clone() => PairState {
                funding_per_unit: FundingPerUnit::new_int(3),
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
            compute_user_equity(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
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
            margin: UsdValue::new_int(10_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(1),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
                btc::DENOM.clone() => Position {
                    size: Quantity::new_int(-1),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), hash_map! {
            eth::DENOM.clone() => PairState {
                funding_per_unit: FundingPerUnit::new_int(3),
                ..Default::default()
            },
            btc::DENOM.clone() => PairState {
                funding_per_unit: FundingPerUnit::new_int(0),
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
            compute_user_equity(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(16_980),
        );
    }

    // collateral=100, ETH long 10 @ entry=2000, oracle=1500
    // pnl = 10 * (1500 - 2000) = -5000
    // equity = 100 + (-5000) - 0 = -4900
    #[test]
    fn equity_negative() {
        let user_state = UserState {
            margin: UsdValue::new_int(100),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), hash_map! {
            eth::DENOM.clone() => PairState {
                funding_per_unit: FundingPerUnit::new_int(0),
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
            compute_user_equity(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(-4900),
        );
    }

    // ---- compute_maintenance_margin tests ----

    #[test]
    fn maintenance_margin_no_positions() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_maintenance_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
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
                    conditional_order_above: None,
                    conditional_order_below: None,
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_maintenance_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
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
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
                btc::DENOM.clone() => Position {
                    size: Quantity::new_int(-1),
                    entry_price: UsdPrice::new_int(50000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
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
            compute_maintenance_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(2500),
        );
    }

    // ---- compute_initial_margin tests ----

    // ETH long 10, oracle=$2000, IMR=10%
    // IM = |10| * 2000 * 0.10 = $2000
    #[test]
    fn initial_margin_single_position() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_initial_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(2000),
        );
    }

    // No positions → $0
    #[test]
    fn initial_margin_no_positions() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_initial_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::ZERO,
        );
    }

    // ETH long 10 + BTC short 1, different IMRs
    // ETH: |10| * 2000 * 0.10 = $2000
    // BTC: |-1| * 50000 * 0.05 = $2500
    // Total = $4500
    #[test]
    fn initial_margin_multiple_positions() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
                btc::DENOM.clone() => Position {
                    size: Quantity::new_int(-1),
                    entry_price: UsdPrice::new_int(50000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
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
            compute_initial_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(4500),
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
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_available_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
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
            margin: UsdValue::new_int(10_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_available_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(12_500),
        );
    }

    // Same as above but reserved=2000
    // available = 12500 - 2000 = 10500
    #[test]
    fn available_margin_with_reserved() {
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            reserved_margin: UsdValue::new_int(2_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_available_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
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
            margin: UsdValue::new_int(100),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(150_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_available_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
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
            margin: UsdValue::new_int(10_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(1),
                    conditional_order_above: None,
                    conditional_order_below: None,
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_available_margin(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(12_480),
        );
    }

    // ---- check_margin tests ----

    /// 100%-fill check fails for a pure opening order at oracle price.
    ///
    /// No existing position. oracle = $50,000, IMR = 5%, taker_fee = 0.1%
    /// Buy 10 BTC (target = oracle = $50,000, pure opening).
    ///
    ///   Post-fill: long 10 @ $50,000. Unrealized PnL = 0.
    ///   margin after fee = $25,200 - $500 = $24,700
    ///   equity = $24,700
    ///   projected_im = |10| * 50,000 * 0.05 = $25,000
    ///   $24,700 < $25,000 → FAILS
    #[test]
    fn margin_check_full_fill_fails() {
        let pair_id: PairId = "perp/btcusd".parse().unwrap();
        let param = Param {
            taker_fee_rates: RateSchedule {
                base: Dimensionless::new_permille(1), // 0.1%
                ..Default::default()
            },
            ..Default::default()
        };
        let taker_state = UserState {
            margin: UsdValue::new_int(25_200),
            ..Default::default()
        };

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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        // Pure opening: closing_size = 0, opening_size = 10.
        // target_price = oracle (no slippage).
        let result = check_margin(
            &mut oracle_querier,
            &pair_id,
            &perp_querier,
            &PairState::default(),
            &taker_state,
            param.taker_fee_rates.base,
            UsdPrice::new_int(50_000),
            Quantity::ZERO,
            Quantity::new_int(10),
        );

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("insufficient margin"),
            "expected margin error, got: {msg}"
        );
    }

    // ---- compute_total_notional tests ----

    #[test]
    fn total_notional_no_positions() {
        let user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_total_notional(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::ZERO,
        );
    }

    // ETH long 10, oracle=$2000
    // notional = |10| * 2000 = $20,000
    #[test]
    fn total_notional_single_position() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert_eq!(
            compute_total_notional(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(20_000),
        );
    }

    // ETH long 10 + BTC short 1
    // ETH: |10| * $2000 = $20,000
    // BTC: |-1| * $50,000 = $50,000
    // Total = $70,000
    #[test]
    fn total_notional_multiple_positions() {
        let user_state = UserState {
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
                btc::DENOM.clone() => Position {
                    size: Quantity::new_int(-1),
                    entry_price: UsdPrice::new_int(50000),
                    entry_funding_per_unit: FundingPerUnit::new_int(0),
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
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
            compute_total_notional(&mut oracle_querier, &perp_querier, &user_state).unwrap(),
            UsdValue::new_int(70_000),
        );
    }
}
