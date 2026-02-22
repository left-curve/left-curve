use {
    super::funding::compute_unrecorded_funding_per_unit,
    crate::NoCachePairQuerier,
    dango_oracle::OracleQuerier,
    dango_types::{
        Ratio, UsdPrice, UsdValue,
        perps::{PairId, PairParam, PairState},
    },
    grug::Timestamp,
};

/// Compute the vault's unrealized PnL for a single trading pair.
///
/// The vault is the counterparty to all traders, so its PnL is the negative
/// of the aggregate trader PnL:
///
/// ```plain
/// vault_pnl = oi_weighted_entry_price - oracle_price * skew
/// ```
///
/// Positive result means the vault is profiting; negative means the vault is
/// losing on this pair.
fn compute_pair_unrealized_pnl(
    pair_state: &PairState,
    oracle_price: UsdPrice,
) -> grug::MathResult<UsdValue> {
    let market_value = pair_state.skew.checked_mul(oracle_price)?;
    pair_state.oi_weighted_entry_price.checked_sub(market_value)
}

/// Compute the vault's unrealized funding for a single trading pair.
///
/// This includes both the recorded portion (already in the accumulator) and
/// funding accrued since the last `accrue_funding` call:
///
/// ```plain
/// recorded  = funding_per_unit * skew - oi_weighted_entry_funding
/// unrecorded = unrecorded_funding_per_unit * skew
/// vault_funding = recorded + unrecorded
/// ```
///
/// Positive result means the vault has earned funding; negative means it owes.
pub fn compute_pair_unrealized_funding(
    pair_state: &PairState,
    pair_param: &PairParam,
    oracle_price: UsdPrice,
    current_time: Timestamp,
) -> grug::MathResult<UsdValue> {
    // Recorded funding already captured in the accumulator.
    let recorded = pair_state
        .skew
        .checked_mul(pair_state.funding_per_unit)?
        .checked_sub(pair_state.oi_weighted_entry_funding)?;

    // Funding accrued since the last accrual call. If no time has elapsed,
    // the unrecorded portion is zero — skip to avoid unnecessary computation
    // (mirrors the early return in `accrue_funding`).
    if current_time == pair_state.last_funding_time {
        return Ok(recorded);
    }

    let (unrecorded_per_unit, _) =
        compute_unrecorded_funding_per_unit(pair_state, pair_param, current_time, oracle_price)?;

    let unrecorded = pair_state.skew.checked_mul(unrecorded_per_unit)?;

    recorded.checked_add(unrecorded)
}

/// Compute the vault's equity (net account value) across all trading pairs.
///
/// ```plain
/// vault_equity = vault_margin_value + Σ(unrealized_pnl) + Σ(unrealized_funding)
/// ```
///
/// Note the sign difference from user equity: user equity *subtracts* funding
/// (trader pays), vault equity *adds* funding (vault receives the net).
pub fn compute_vault_equity(
    vault_margin_value: UsdValue,
    pair_ids: &[PairId],
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
    current_time: Timestamp,
) -> anyhow::Result<UsdValue> {
    let mut total_pnl = UsdValue::ZERO;
    let mut total_funding = UsdValue::ZERO;

    for pair_id in pair_ids {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_state = pair_querier.query_pair_state(pair_id)?;
        let pair_param = pair_querier.query_pair_param(pair_id)?;

        total_pnl =
            total_pnl.checked_add(compute_pair_unrealized_pnl(&pair_state, oracle_price)?)?;
        total_funding = total_funding.checked_add(compute_pair_unrealized_funding(
            &pair_state,
            &pair_param,
            oracle_price,
            current_time,
        )?)?;
    }

    Ok(vault_margin_value
        .checked_add(total_pnl)?
        .checked_add(total_funding)?)
}

/// Returns true if the vault is distressed enough to trigger auto-deleveraging.
///
/// ADL is triggerable when the vault's equity falls below a fraction of total
/// open notional:
///
/// ```plain
/// vault_equity < (long_oi + short_oi) * oracle_price * adl_trigger_ratio
/// ```
///
/// Early-returns `false` when there are no pairs (no open interest → no
/// distress).
pub fn is_adl_triggerable(
    vault_margin_value: UsdValue,
    pair_ids: &[PairId],
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
    current_time: Timestamp,
    adl_trigger_ratio: Ratio<UsdValue>,
) -> anyhow::Result<bool> {
    if pair_ids.is_empty() {
        return Ok(false);
    }

    let vault_equity = compute_vault_equity(
        vault_margin_value,
        pair_ids,
        pair_querier,
        oracle_querier,
        current_time,
    )?;

    let mut total_open_notional = UsdValue::ZERO;

    for pair_id in pair_ids {
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
        let pair_state = pair_querier.query_pair_state(pair_id)?;

        let pair_oi = pair_state.long_oi.checked_add(pair_state.short_oi)?;
        let pair_notional = pair_oi.checked_mul(oracle_price)?;

        total_open_notional = total_open_notional.checked_add(pair_notional)?;
    }

    let threshold = total_open_notional.checked_mul(adl_trigger_ratio)?;

    Ok(vault_equity < threshold)
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
            perps::{PairParam, PairState},
        },
        grug::{Timestamp, Udec128, hash_map},
        std::collections::HashMap,
        test_case::test_case,
    };

    // ---- compute_pair_unrealized_pnl tests ----

    // vault_pnl = oi_weighted_entry_price - oracle_price * skew
    #[test_case( 10, 20_000, 2500, -5000 ; "positive skew vault loses")]
    #[test_case( 10, 20_000, 1500,  5000 ; "positive skew vault profits")]
    #[test_case(-10, -20_000, 2500, 5000 ; "negative skew vault profits")]
    #[test_case(  0,       0, 2000,    0 ; "zero skew")]
    fn compute_pair_unrealized_pnl_works(
        skew: i128,
        oi_weighted_entry_price: i128,
        oracle_price: i128,
        expected: i128,
    ) {
        let pair_state = PairState {
            skew: HumanAmount::new(skew),
            oi_weighted_entry_price: UsdValue::new(oi_weighted_entry_price),
            ..Default::default()
        };

        assert_eq!(
            compute_pair_unrealized_pnl(&pair_state, UsdPrice::new_int(oracle_price)).unwrap(),
            UsdValue::new(expected),
        );
    }

    // ---- compute_pair_unrealized_funding tests ----

    #[test]
    fn funding_no_delta() {
        let pair_state = PairState {
            skew: HumanAmount::new(10),
            funding_per_unit: Ratio::new_int(0),
            oi_weighted_entry_funding: UsdValue::new(0),
            last_funding_time: Timestamp::from_seconds(1_000_000),
            ..Default::default()
        };
        let pair_param = PairParam::default();
        let oracle_price = UsdPrice::new_int(2000);

        // No time elapsed → no unrecorded funding, and recorded is zero.
        assert_eq!(
            compute_pair_unrealized_funding(
                &pair_state,
                &pair_param,
                oracle_price,
                Timestamp::from_seconds(1_000_000),
            )
            .unwrap(),
            UsdValue::ZERO,
        );
    }

    #[test]
    fn funding_recorded_only() {
        let pair_state = PairState {
            skew: HumanAmount::new(10),
            funding_per_unit: Ratio::new_int(100),
            oi_weighted_entry_funding: UsdValue::new(500),
            last_funding_time: Timestamp::from_seconds(1_000_000),
            ..Default::default()
        };
        let pair_param = PairParam::default();
        let oracle_price = UsdPrice::new_int(2000);

        // recorded = 100 * 10 - 500 = 500, no time elapsed → unrecorded = 0
        assert_eq!(
            compute_pair_unrealized_funding(
                &pair_state,
                &pair_param,
                oracle_price,
                Timestamp::from_seconds(1_000_000),
            )
            .unwrap(),
            UsdValue::new(500),
        );
    }

    #[test]
    fn funding_with_unrecorded() {
        // Use the same funding parameters as funding.rs tests:
        //   skew_scale=1000, max_funding_velocity=0.1/day², max_abs_funding_rate=0.05/day
        //   oracle_price=100, funding_rate=0, elapsed=1 day, skew=1000
        //
        // From funding.rs: unrecorded_per_unit = 2.5 (raw 2_500_000)
        // unrecorded = 2.5 * 1000 = 2500
        // recorded = 0 * 1000 - 0 = 0
        // total = 2500
        let pair_state = PairState {
            skew: HumanAmount::new(1000),
            funding_per_unit: Ratio::new_raw(0),
            oi_weighted_entry_funding: UsdValue::new(0),
            funding_rate: Ratio::new_raw(0),
            last_funding_time: Timestamp::from_seconds(1_000_000),
            ..Default::default()
        };
        let pair_param = PairParam {
            skew_scale: Ratio::new_int(1000),
            max_funding_velocity: Ratio::new_raw(100_000),
            max_abs_funding_rate: Ratio::new_raw(50_000),
            ..Default::default()
        };
        let oracle_price = UsdPrice::new_int(100);
        let current_time = Timestamp::from_seconds(1_000_000 + 86400);

        assert_eq!(
            compute_pair_unrealized_funding(&pair_state, &pair_param, oracle_price, current_time)
                .unwrap(),
            UsdValue::new(2500),
        );
    }

    // ---- compute_vault_equity tests ----

    #[test]
    fn vault_equity_no_pairs() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert_eq!(
            compute_vault_equity(
                UsdValue::new(10_000),
                &[],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(0),
            )
            .unwrap(),
            UsdValue::new(10_000),
        );
    }

    // vault_margin=10000, ETH: skew=10, oi_weighted_entry=20000, oracle=2500
    // pnl = 20000 - 2500*10 = -5000, no funding
    // equity = 10000 + (-5000) + 0 = 5000
    #[test]
    fn vault_equity_single_pair_pnl_only() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: HumanAmount::new(10),
                    oi_weighted_entry_price: UsdValue::new(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
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
            compute_vault_equity(
                UsdValue::new(10_000),
                &[eth::DENOM.clone()],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(0),
            )
            .unwrap(),
            UsdValue::new(5000),
        );
    }

    // vault_margin=10000, ETH: skew=10, oi_weighted_entry=20000, oracle=2500
    // pnl = 20000 - 25000 = -5000
    // funding: funding_per_unit=3, oi_weighted_entry_funding=10, no time elapsed
    //   recorded = 3*10 - 10 = 20
    // equity = 10000 + (-5000) + 20 = 5020
    #[test]
    fn vault_equity_single_pair_with_funding() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: HumanAmount::new(10),
                    oi_weighted_entry_price: UsdValue::new(20_000),
                    funding_per_unit: Ratio::new_int(3),
                    oi_weighted_entry_funding: UsdValue::new(10),
                    last_funding_time: Timestamp::from_seconds(100),
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
            compute_vault_equity(
                UsdValue::new(10_000),
                &[eth::DENOM.clone()],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(100),
            )
            .unwrap(),
            UsdValue::new(5020),
        );
    }

    // Two pairs:
    //   ETH: skew=10, oi_weighted_entry=20000, oracle=2500
    //     pnl = 20000 - 25000 = -5000, no funding
    //   BTC: skew=-1, oi_weighted_entry=-50000, oracle=48000
    //     pnl = -50000 - 48000*(-1) = -50000 + 48000 = -2000, no funding
    //   equity = 10000 + (-5000) + (-2000) = 3000
    #[test]
    fn vault_equity_multiple_pairs() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
                btc::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: HumanAmount::new(10),
                    oi_weighted_entry_price: UsdValue::new(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairState {
                    skew: HumanAmount::new(-1),
                    oi_weighted_entry_price: UsdValue::new(-50_000),
                    last_funding_time: Timestamp::from_seconds(0),
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
            btc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(4_800_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        assert_eq!(
            compute_vault_equity(
                UsdValue::new(10_000),
                &[eth::DENOM.clone(), btc::DENOM.clone()],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(0),
            )
            .unwrap(),
            UsdValue::new(3000),
        );
    }

    // vault_margin=100, ETH: skew=10, oi_weighted_entry=20000, oracle=2500
    // pnl = 20000 - 25000 = -5000
    // equity = 100 + (-5000) = -4900
    #[test]
    fn vault_equity_negative() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: HumanAmount::new(10),
                    oi_weighted_entry_price: UsdValue::new(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
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
            compute_vault_equity(
                UsdValue::new(100),
                &[eth::DENOM.clone()],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(0),
            )
            .unwrap(),
            UsdValue::new(-4900),
        );
    }

    // ---- is_adl_triggerable tests ----

    #[test]
    fn adl_no_pairs() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert!(
            !is_adl_triggerable(
                UsdValue::new(10_000),
                &[],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(0),
                Ratio::new_permille(500),
            )
            .unwrap()
        );
    }

    // vault_margin=100000, ETH: skew=10, long_oi=15, short_oi=5, oracle=2000
    // oi_weighted_entry=20000 (no pnl: 20000 - 2000*10 = 0)
    // vault_equity = 100000 + 0 + 0 = 100000
    // total_open_notional = (15 + 5) * 2000 = 40000
    // threshold = 40000 * 0.5 = 20000
    // 100000 >= 20000 → not triggerable
    #[test]
    fn adl_healthy_vault() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: HumanAmount::new(10),
                    long_oi: HumanAmount::new(15),
                    short_oi: HumanAmount::new(5),
                    oi_weighted_entry_price: UsdValue::new(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
                    ..Default::default()
                },
            },
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert!(
            !is_adl_triggerable(
                UsdValue::new(100_000),
                &[eth::DENOM.clone()],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(0),
                Ratio::new_permille(500),
            )
            .unwrap()
        );
    }

    // vault_equity exactly equals threshold → not triggerable (strict <)
    // ETH: skew=10, long_oi=15, short_oi=5, oracle=2000
    // oi_weighted_entry=20000 (no pnl)
    // total_open_notional = (15+5)*2000 = 40000
    // threshold = 40000 * 0.5 = 20000
    // Need vault_equity = 20000 → vault_margin = 20000
    #[test]
    fn adl_at_boundary() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: HumanAmount::new(10),
                    long_oi: HumanAmount::new(15),
                    short_oi: HumanAmount::new(5),
                    oi_weighted_entry_price: UsdValue::new(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
                    ..Default::default()
                },
            },
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert!(
            !is_adl_triggerable(
                UsdValue::new(20_000),
                &[eth::DENOM.clone()],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(0),
                Ratio::new_permille(500),
            )
            .unwrap()
        );
    }

    // vault_margin=100, ETH: skew=10, long_oi=15, short_oi=5, oracle=2500
    // oi_weighted_entry=20000 → pnl = 20000 - 25000 = -5000
    // vault_equity = 100 + (-5000) = -4900
    // total_open_notional = (15+5)*2500 = 50000
    // threshold = 50000 * 0.5 = 25000
    // -4900 < 25000 → triggerable
    #[test]
    fn adl_distressed_vault() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: HumanAmount::new(10),
                    long_oi: HumanAmount::new(15),
                    short_oi: HumanAmount::new(5),
                    oi_weighted_entry_price: UsdValue::new(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
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

        assert!(
            is_adl_triggerable(
                UsdValue::new(100),
                &[eth::DENOM.clone()],
                &pair_querier,
                &mut oracle_querier,
                Timestamp::from_seconds(0),
                Ratio::new_permille(500),
            )
            .unwrap()
        );
    }
}
