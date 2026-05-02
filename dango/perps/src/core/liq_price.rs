use {
    crate::{
        core::{compute_position_unrealized_funding, compute_position_unrealized_pnl},
        querier::NoCachePerpQuerier,
    },
    dango_oracle::OracleQuerier,
    dango_order_book::{PairId, UsdPrice, UsdValue},
    dango_types::perps::UserState,
};

/// Compute the liquidation price for a single position in a cross-margin account.
///
/// This is the oracle price of the given pair at which the account-level
/// liquidation condition (`equity < maintenance_margin`) triggers, assuming all
/// other pair prices remain constant (partial-derivative approach).
///
/// Given:
///
/// ```plain
/// equity(p) = M + other_pnl + sⱼ*(p - epⱼ) - total_funding
/// mm(p)     = other_mm + |sⱼ| * p * mmrⱼ
/// ```
///
/// Setting `equity(p) = mm(p)` and solving for `p`:
///
/// ```plain
/// C = M + other_pnl - sⱼ*epⱼ - total_funding - other_mm
/// p = C / (|sⱼ|*mmrⱼ - sⱼ)
/// ```
///
/// Returns `None` when:
/// - The computed price is non-positive (the position alone cannot trigger liquidation).
/// - The position does not exist for the given pair.
pub fn compute_liquidation_price(
    pair_id: &PairId,
    user_state: &UserState,
    oracle_querier: &mut OracleQuerier,
    perp_querier: &NoCachePerpQuerier,
) -> anyhow::Result<Option<UsdPrice>> {
    let Some(target) = user_state.positions.get(pair_id) else {
        return Ok(None);
    };

    let target_param = perp_querier.query_pair_param(pair_id)?;

    // Accumulate contributions from all positions.
    let mut other_pnl = UsdValue::ZERO;
    let mut total_funding = UsdValue::ZERO;
    let mut other_mm = UsdValue::ZERO;

    for (pid, position) in &user_state.positions {
        let oracle_price = oracle_querier.query_price_for_perps(pid)?;
        let pair_state = perp_querier.query_pair_state(pid)?;

        let funding = compute_position_unrealized_funding(position, &pair_state)?;
        total_funding.checked_add_assign(funding)?;

        if pid != pair_id {
            let pnl = compute_position_unrealized_pnl(position, oracle_price)?;
            other_pnl.checked_add_assign(pnl)?;

            let pair_param = perp_querier.query_pair_param(pid)?;
            let mm = position
                .size
                .checked_abs()?
                .checked_mul(oracle_price)?
                .checked_mul(pair_param.maintenance_margin_ratio)?;
            other_mm.checked_add_assign(mm)?;
        }
    }

    // C = margin + other_pnl - sⱼ*epⱼ - total_funding - other_mm
    let size_times_entry = target.size.checked_mul(target.entry_price)?;
    let c = user_state
        .margin
        .checked_add(other_pnl)?
        .checked_sub(size_times_entry)?
        .checked_sub(total_funding)?
        .checked_sub(other_mm)?;

    // denom = |sⱼ|*mmrⱼ - sⱼ
    let abs_size_times_mmr = target
        .size
        .checked_abs()?
        .checked_mul(target_param.maintenance_margin_ratio)?;
    let denom = abs_size_times_mmr.checked_sub(target.size)?;

    let liq_price = c.checked_div(denom)?;

    if liq_price.is_positive() {
        Ok(Some(liq_price))
    } else {
        Ok(None)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::{Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue},
        dango_types::{
            constants::{btc, eth},
            oracle::PrecisionedPrice,
            perps::{PairParam, PairState, Position},
        },
        grug::{Timestamp, Udec128, btree_map, hash_map},
        std::collections::HashMap,
    };

    /// Helper: build an oracle price entry for the mock querier.
    ///
    /// `price` is the integer dollar price (e.g. 2000 for $2,000).
    fn oracle_entry(price: u128) -> PrecisionedPrice {
        PrecisionedPrice::new(
            Udec128::new_percent(price * 100),
            Timestamp::from_seconds(0),
            18,
        )
    }

    fn pair_param_with_mmr(mmr_permille: i128) -> PairParam {
        PairParam {
            maintenance_margin_ratio: Dimensionless::new_permille(mmr_permille),
            ..Default::default()
        }
    }

    // ---- Single long position ----
    //
    // margin=10_000, long 10 ETH @ entry=2000, oracle=2000, mmr=5%, no funding
    //
    // C = 10000 + 0 - 10*2000 - 0 - 0 = -10000
    // denom = 10*0.05 - 10 = -9.5
    // liq_price = -10000 / -9.5 = 1052.631578... (truncated at 6 decimals)
    #[test]
    fn single_long_no_funding() {
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! { eth::DENOM.clone() => pair_param_with_mmr(50) },
            hash_map! { eth::DENOM.clone() => PairState::default() },
        );
        let mut oracle_querier =
            OracleQuerier::new_mock(hash_map! { eth::DENOM.clone() => oracle_entry(2000) });

        let liq =
            compute_liquidation_price(&eth::DENOM, &user_state, &mut oracle_querier, &perp_querier)
                .unwrap()
                .expect("should have a liquidation price");

        assert_eq!(liq, UsdPrice::new_raw(1_052_631_578));
    }

    // ---- Single short position ----
    //
    // margin=5_000, short 5 ETH @ entry=2000, oracle=2000, mmr=5%, no funding
    //
    // C = 5000 - (-5)*2000 = 15000
    // denom = 5*0.05 - (-5) = 5.25
    // liq_price = 15000 / 5.25 = 2857.142857... (truncated at 6 decimals)
    #[test]
    fn single_short_no_funding() {
        let user_state = UserState {
            margin: UsdValue::new_int(5_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(-5),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! { eth::DENOM.clone() => pair_param_with_mmr(50) },
            hash_map! { eth::DENOM.clone() => PairState::default() },
        );
        let mut oracle_querier =
            OracleQuerier::new_mock(hash_map! { eth::DENOM.clone() => oracle_entry(2000) });

        let liq =
            compute_liquidation_price(&eth::DENOM, &user_state, &mut oracle_querier, &perp_querier)
                .unwrap()
                .expect("should have a liquidation price");

        assert_eq!(liq, UsdPrice::new_raw(2_857_142_857));
    }

    // ---- Funding pushes liq price closer ----
    //
    // Same as single_long_no_funding, but funding_delta=5 means the long owes
    // 10*5 = 50 USD extra, pulling the liq price up.
    //
    // C = 10000 + 0 - 10*2000 - 50 - 0 = -10050
    // denom = -9.5
    // liq_price = -10050 / -9.5 = 1057.894736...
    #[test]
    fn long_with_funding() {
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! { eth::DENOM.clone() => pair_param_with_mmr(50) },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    funding_per_unit: FundingPerUnit::new_int(5),
                    ..Default::default()
                },
            },
        );
        let mut oracle_querier =
            OracleQuerier::new_mock(hash_map! { eth::DENOM.clone() => oracle_entry(2000) });

        let liq =
            compute_liquidation_price(&eth::DENOM, &user_state, &mut oracle_querier, &perp_querier)
                .unwrap()
                .expect("should have a liquidation price");

        // C = -10000 - 50 = -10050, denom = -9.5
        // liq = 10050 / 9.5 = 1057.894736... (truncated at 6 decimals)
        assert_eq!(liq, UsdPrice::new_raw(1_057_894_736));
    }

    // ---- Cross-margin: other position's profit raises liq threshold ----
    //
    // margin=1000, long 10 ETH @ 2000, oracle=2000, mmr=5%
    //             long 1 BTC @ 50000, oracle=55000, mmr=5%
    //
    // BTC pnl = 1*(55000 - 50000) = 5000
    // BTC mm = 1*55000*0.05 = 2750
    // BTC funding = 0
    //
    // other_pnl = 5000, other_mm = 2750, total_funding = 0
    //
    // C = 1000 + 5000 - 10*2000 - 0 - 2750 = -16750
    // denom = 10*0.05 - 10 = -9.5
    // liq_price = -16750 / -9.5 = 1763.157894...
    //
    // Without the BTC position (margin=1000 alone):
    // C = 1000 - 20000 = -19000
    // liq_price = -19000 / -9.5 = 2000 (exactly at entry — user has no margin
    // headroom without the BTC profit)
    #[test]
    fn cross_margin_other_profit_helps() {
        let user_state = UserState {
            margin: UsdValue::new_int(1_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
                btc::DENOM.clone() => Position {
                    size: Quantity::new_int(1),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => pair_param_with_mmr(50),
                btc::DENOM.clone() => pair_param_with_mmr(50),
            },
            hash_map! {
                eth::DENOM.clone() => PairState::default(),
                btc::DENOM.clone() => PairState::default(),
            },
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => oracle_entry(2000),
            btc::DENOM.clone() => oracle_entry(55_000),
        });

        let liq =
            compute_liquidation_price(&eth::DENOM, &user_state, &mut oracle_querier, &perp_querier)
                .unwrap()
                .expect("should have a liquidation price");

        // C = 1000 + 5000 - 20000 - 0 - 2750 = -16750, denom = -9.5
        // liq = 16750 / 9.5 = 1763.157894... (truncated at 6 decimals)
        assert_eq!(liq, UsdPrice::new_raw(1_763_157_894));
    }

    // ---- Position can't cause liquidation alone → None ----
    //
    // margin=100_000, long 1 ETH @ 2000, oracle=2000, mmr=5%
    //
    // C = 100000 - 2000 = 98000
    // denom = 0.05 - 1 = -0.95
    // liq_price = 98000 / -0.95 = -103157.89... (negative → None)
    //
    // The margin is so large relative to the position that ETH would have to go
    // negative to trigger liquidation — impossible.
    #[test]
    fn large_margin_returns_none() {
        let user_state = UserState {
            margin: UsdValue::new_int(100_000),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(1),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! { eth::DENOM.clone() => pair_param_with_mmr(50) },
            hash_map! { eth::DENOM.clone() => PairState::default() },
        );
        let mut oracle_querier =
            OracleQuerier::new_mock(hash_map! { eth::DENOM.clone() => oracle_entry(2000) });

        let result =
            compute_liquidation_price(&eth::DENOM, &user_state, &mut oracle_querier, &perp_querier)
                .unwrap();

        assert!(result.is_none());
    }

    // ---- Missing position → None ----
    #[test]
    fn missing_position_returns_none() {
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        let result =
            compute_liquidation_price(&eth::DENOM, &user_state, &mut oracle_querier, &perp_querier)
                .unwrap();

        assert!(result.is_none());
    }

    // ---- Already underwater: liq price above current oracle for long ----
    //
    // margin=100, long 10 ETH @ 2000, oracle=1500, mmr=5%
    //   equity = 100 + 10*(1500-2000) = 100 - 5000 = -4900
    //   mm = 10*1500*0.05 = 750
    //   Already liquidatable (-4900 < 750).
    //
    // C = 100 - 20000 = -19900
    // denom = -9.5
    // liq_price = -19900 / -9.5 = 2094.736842...
    //
    // The liq price is above the current oracle price (1500), meaning the user
    // is already past the threshold. This is valid — it tells us "you'd need the
    // price to recover above 2094 to escape liquidation."
    #[test]
    fn already_underwater_still_returns_price() {
        let user_state = UserState {
            margin: UsdValue::new_int(100),
            positions: btree_map! {
                eth::DENOM.clone() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(2000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(
            hash_map! { eth::DENOM.clone() => pair_param_with_mmr(50) },
            hash_map! { eth::DENOM.clone() => PairState::default() },
        );
        let mut oracle_querier =
            OracleQuerier::new_mock(hash_map! { eth::DENOM.clone() => oracle_entry(1500) });

        let liq =
            compute_liquidation_price(&eth::DENOM, &user_state, &mut oracle_querier, &perp_querier)
                .unwrap()
                .expect("should still have a liquidation price");

        // liq = 19900 / 9.5 = 2094.736842... (above current oracle of 1500)
        assert_eq!(liq, UsdPrice::new_raw(2_094_736_842));
    }
}
