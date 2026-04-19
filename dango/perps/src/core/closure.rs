use {
    crate::{
        core::{compute_maintenance_margin, compute_user_equity},
        querier::NoCachePerpQuerier,
    },
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, UsdPrice, UsdValue,
        perps::{PairId, PairParam, UserState},
    },
    grug::MathResult,
    std::collections::BTreeMap,
};

/// Returns true if the user is eligible for liquidation.
/// Also returns equity and maintenance margin for error logging purpose.
///
/// A user is liquidatable when their equity (collateral + unrealized PnL
/// - accrued funding) falls below their total maintenance margin.
///
/// A user with no open positions is never liquidatable.
pub fn is_liquidatable(
    oracle_querier: &mut OracleQuerier,
    perp_querier: &NoCachePerpQuerier,
    user_state: &UserState,
) -> anyhow::Result<(bool, UsdValue, UsdValue)> {
    let equity = compute_user_equity(oracle_querier, perp_querier, user_state)?;
    let maintenance_margin = compute_maintenance_margin(oracle_querier, perp_querier, user_state)?;

    Ok((equity < maintenance_margin, equity, maintenance_margin))
}

/// A policy for selecting which position(s) to close during liquidation.
/// We start from the position that contributes the most to maintenance margin
/// and go down, until the maintenance margin deficit is covered.
///
/// Returns:
///
/// - A vector of (pair_id, close_size) tuples.
pub fn compute_close_schedule(
    user_state: &UserState,
    pair_params: &BTreeMap<PairId, PairParam>,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    deficit: UsdValue,
) -> MathResult<Vec<(PairId, Quantity)>> {
    let mut deficit = deficit;

    // Build (mm_contribution, pair_id) list, sorted descending by MM.
    let mut mm_entries = Vec::new();

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_prices[pair_id];
        let pair_param = &pair_params[pair_id];

        let mm_contribution = position
            .size
            .checked_abs()?
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.maintenance_margin_ratio)?;

        mm_entries.push((mm_contribution, pair_id.clone()));
    }

    // Sort by MM contribution descending.
    mm_entries.sort_by_key(|(a, _)| std::cmp::Reverse(*a));

    // Build the close schedule.
    let mut schedule = Vec::new();

    for (_, pair_id) in &mm_entries {
        if deficit <= UsdValue::ZERO {
            break;
        }

        let position = &user_state.positions[pair_id];
        let oracle_price = oracle_prices[pair_id];
        let pair_param = &pair_params[pair_id];
        let abs_size = position.size.checked_abs()?;

        // close_amount = min(ceil(deficit / (P × mmr)), |size|)
        //
        // Ceiling rounding is load-bearing: with floor division, a sub-ULP
        // deficit collapses `close_amount` to zero, leaves the schedule
        // empty, and causes `liquidate` to silently exit with no events.
        // Ceil guarantees at least 1 ULP of progress whenever `deficit > 0`.
        let close_amount = {
            let denominator = oracle_price.checked_mul(pair_param.maintenance_margin_ratio)?;
            deficit.checked_div_ceil(denominator)?.min(abs_size)
        };

        // close_size = -sign(size) × close_amount (opposite direction to close)
        let close_size = if position.size.is_positive() {
            close_amount.checked_neg()?
        } else {
            close_amount
        };

        deficit = {
            let mm_to_remove = close_amount
                .checked_mul(oracle_price)?
                .checked_mul(pair_param.maintenance_margin_ratio)?;
            deficit.checked_sub(mm_to_remove)?.max(UsdValue::ZERO)
        };

        if close_size.is_non_zero() {
            schedule.push((pair_id.clone(), close_size));
        }

        // If the full position is being closed, deficit should be exactly cleared for
        // that contribution. But if we're doing partial, deficit might still be > 0
        // only if there are precision issues. The guard at the top handles this.
    }

    Ok(schedule)
}

/// Compute the user's equity from in-memory state and accumulated PnL/fees.
///
/// This is the "raw" equity computation used during liquidation, where oracle
/// prices are already resolved and funding has been settled into `user_pnl`.
///
/// ```plain
/// equity = margin + user_pnl - user_fees + Σ(size × (oracle - entry))
/// ```
pub fn compute_user_equity_with_pnl(
    user_state: &UserState,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    user_pnl: UsdValue,
    user_fees: UsdValue,
) -> MathResult<UsdValue> {
    let mut equity = user_state
        .margin
        .checked_add(user_pnl)?
        .checked_sub(user_fees)?;

    for (pid, pos) in &user_state.positions {
        let oracle_price = oracle_prices[pid];
        let unrealized = pos
            .size
            .checked_mul(oracle_price.checked_sub(pos.entry_price)?)?;

        equity.checked_add_assign(unrealized)?;
    }

    Ok(equity)
}

/// Compute the bankruptcy price for a position being closed during liquidation.
///
/// This is the fill price at which the user's total equity would be exactly
/// zero after closing `close_amount` of the position.
///
/// For longs:  bp = oracle_price - equity / close_amount
/// For shorts: bp = oracle_price + equity / close_amount
pub fn compute_bankruptcy_price(
    user_state: &UserState,
    pair_id: &PairId,
    close_amount: Quantity,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    user_pnl: UsdValue,
    user_fees: UsdValue,
) -> MathResult<UsdPrice> {
    let equity = compute_user_equity_with_pnl(user_state, oracle_prices, user_pnl, user_fees)?;

    let position = &user_state.positions[pair_id];
    let oracle_price = oracle_prices[pair_id];
    let offset = equity.checked_div(close_amount)?;

    if position.size.is_positive() {
        oracle_price.checked_sub(offset)
    } else {
        oracle_price.checked_add(offset)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue,
            constants::eth,
            oracle::PrecisionedPrice,
            perps::{PairParam, PairState, Position},
        },
        grug::{Timestamp, Udec128, btree_map, hash_map},
        std::collections::HashMap,
    };

    fn pair_btc() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn pair_eth() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    fn btc_pair_param() -> PairParam {
        PairParam {
            maintenance_margin_ratio: Dimensionless::new_permille(50), // 5%
            ..Default::default()
        }
    }

    fn eth_pair_param() -> PairParam {
        PairParam {
            maintenance_margin_ratio: Dimensionless::new_permille(50), // 5%
            ..Default::default()
        }
    }

    // ------------------------ `is_liquidatable` tests ------------------------

    #[test]
    fn is_liquidatable_no_positions() {
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_mock(HashMap::new(), HashMap::new());
        let mut oracle_querier = OracleQuerier::new_mock(HashMap::new());

        assert!(
            !is_liquidatable(&mut oracle_querier, &perp_querier, &user_state)
                .map(|(is, ..)| is)
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert!(
            !is_liquidatable(&mut oracle_querier, &perp_querier, &user_state)
                .map(|(is, ..)| is)
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
            margin: UsdValue::new_int(1_000),
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(200_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert!(
            !is_liquidatable(&mut oracle_querier, &perp_querier, &user_state)
                .map(|(is, ..)| is)
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
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(150_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        assert!(
            is_liquidatable(&mut oracle_querier, &perp_querier, &user_state)
                .map(|(is, ..)| is)
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
                margin: UsdValue::new_int(collateral),
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
            );
            let oracle_querier = OracleQuerier::new_mock(hash_map! {
                eth::DENOM.clone() => PrecisionedPrice::new(
                    Udec128::new_percent(200_000),
                    Timestamp::from_seconds(0),
                    18,
                ),
            });
            (user_state, perp_querier, oracle_querier)
        };

        // Case 1: healthy despite funding
        let (us, pq, mut oq) = make_fixtures(10_000);
        assert!(
            !is_liquidatable(&mut oq, &pq, &us)
                .map(|(is, ..)| is)
                .unwrap()
        );

        // Case 2: funding pushes equity below maintenance margin
        let (us, pq, mut oq) = make_fixtures(900);
        assert!(
            is_liquidatable(&mut oq, &pq, &us)
                .map(|(is, ..)| is)
                .unwrap()
        );
    }

    // -------------------- `compute_close_schedule` tests ---------------------

    /// Single long position, deficit exceeds its MM → full close.
    ///
    /// Position: long 10 BTC @ oracle $47,000, MMR = 5%
    /// MM = 10 * 47000 * 0.05 = 23,500
    /// deficit = 30,000 > MM → close all 10 BTC
    #[test]
    fn single_pair_full_close() {
        let user_state = UserState {
            positions: btree_map! {
                pair_btc() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };

        let pair_params = btree_map! { pair_btc() => btc_pair_param() };
        let oracle_prices = btree_map! { pair_btc() => UsdPrice::new_int(47_000) };

        let schedule = compute_close_schedule(
            &user_state,
            &pair_params,
            &oracle_prices,
            UsdValue::new_int(30_000),
        )
        .unwrap();

        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0].0, pair_btc());
        // Closing a long → negative close_size
        assert_eq!(schedule[0].1, Quantity::new_int(-10));
    }

    /// Two pairs, BTC has larger MM → processed first, both fully closed.
    ///
    /// BTC: long 1 @ oracle $47,000, MMR 5% → MM = 2,350
    /// ETH: long 10 @ oracle $2,800, MMR 5% → MM = 1,400
    /// Total MM = 3,750
    /// deficit = 4,750 > total → close both, BTC first
    #[test]
    fn multi_pair_largest_mm_first() {
        let user_state = UserState {
            positions: btree_map! {
                pair_btc() => Position {
                    size: Quantity::new_int(1),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
                pair_eth() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(3_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };

        let pair_params = btree_map! {
            pair_btc() => btc_pair_param(),
            pair_eth() => eth_pair_param(),
        };
        let oracle_prices = btree_map! {
            pair_btc() => UsdPrice::new_int(47_000),
            pair_eth() => UsdPrice::new_int(2_800),
        };

        let schedule = compute_close_schedule(
            &user_state,
            &pair_params,
            &oracle_prices,
            UsdValue::new_int(4_750),
        )
        .unwrap();

        // Both positions closed.
        assert_eq!(schedule.len(), 2);
        // BTC has larger MM (2350 > 1400) → first.
        assert_eq!(schedule[0].0, pair_btc());
        assert_eq!(schedule[0].1, Quantity::new_int(-1));
        assert_eq!(schedule[1].0, pair_eth());
        assert_eq!(schedule[1].1, Quantity::new_int(-10));
    }

    /// Deficit smaller than one position's full MM → partial close only.
    ///
    /// BTC: long 10 @ oracle $50,000, MMR 5%
    /// MM per unit = 50,000 * 0.05 = 2,500
    /// deficit = 5,000 → close_amount = 5000 / 2500 = 2  (partial)
    #[test]
    fn partial_close() {
        let user_state = UserState {
            positions: btree_map! {
                pair_btc() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };

        let pair_params = btree_map! { pair_btc() => btc_pair_param() };
        let oracle_prices = btree_map! { pair_btc() => UsdPrice::new_int(50_000) };

        let schedule = compute_close_schedule(
            &user_state,
            &pair_params,
            &oracle_prices,
            UsdValue::new_int(5_000),
        )
        .unwrap();

        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0].0, pair_btc());
        // Only 2 of 10 BTC closed
        assert_eq!(schedule[0].1, Quantity::new_int(-2));
    }

    /// Regression for the silent-exit `liquidate` bug observed on mainnet.
    ///
    /// When the deficit is smaller than one ULP of the denominator
    /// `oracle_price × mmr`, floor division of `deficit / denominator` used
    /// to collapse `close_amount` to zero, leaving `compute_close_schedule`
    /// to return an empty `Vec`. Combined with a liquidatable user that has
    /// no resting orders or conditionals, the outer `liquidate` handler
    /// would then write unchanged state and return `Ok` with an empty
    /// `EventBuilder` — a successful tx that emitted no events.
    ///
    /// `compute_close_schedule` now uses ceiling division (matching the doc
    /// comment), which guarantees at least one ULP of close size whenever the
    /// user is liquidatable.
    ///
    /// Setup: long 1 BTC, oracle $60k, mmr 5% → denominator = $3,000.
    /// Deficit = $0.001 (raw 1_000 in `Dec128_6`'s 6-decimal representation).
    ///
    /// - `floor(deficit / denominator) = floor(1_000 × 10⁶ / 3_000_000_000) = floor(1/3) = 0` (old, buggy)
    /// - `ceil (deficit / denominator) = ceil (1/3) = 1` raw ULP of `Quantity` (new)
    #[test]
    fn sub_ulp_deficit_produces_one_ulp_close() {
        let user_state = UserState {
            positions: btree_map! {
                pair_btc() => Position {
                    size: Quantity::new_int(1),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                },
            },
            ..Default::default()
        };

        let pair_params = btree_map! { pair_btc() => btc_pair_param() };
        let oracle_prices = btree_map! { pair_btc() => UsdPrice::new_int(60_000) };

        // $0.001 — one milli-dollar, well below `denominator × 1 ULP = $0.003`.
        let deficit = UsdValue::new_raw(1_000);

        let schedule =
            compute_close_schedule(&user_state, &pair_params, &oracle_prices, deficit).unwrap();

        // With ceil division, the schedule contains exactly one entry of the
        // smallest representable close size (1 raw ULP = 0.000001 Quantity),
        // in the opposite direction of the long position.
        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0].0, pair_btc());
        assert_eq!(schedule[0].1, Quantity::new_raw(-1));
    }

    // ==================== `compute_bankruptcy_price` tests ====================

    #[test]
    fn bankruptcy_price_long_single_position() {
        // Long 1 BTC @ $50k, margin $3k, oracle $46k.
        // Equity = 3k + 1*(46k - 50k) = -1k.
        // bp = 46k - (-1k)/1 = 47k.
        let user_state = UserState {
            margin: UsdValue::new_int(3_000),
            positions: BTreeMap::from([(pair_btc(), Position {
                size: Quantity::new_int(1),
                entry_price: UsdPrice::new_int(50_000),
                entry_funding_per_unit: FundingPerUnit::ZERO,
                conditional_order_above: None,
                conditional_order_below: None,
            })]),
            ..Default::default()
        };

        let oracle_prices = BTreeMap::from([(pair_btc(), UsdPrice::new_int(46_000))]);

        let bp = compute_bankruptcy_price(
            &user_state,
            &pair_btc(),
            Quantity::new_int(1),
            &oracle_prices,
            UsdValue::ZERO,
            UsdValue::ZERO,
        )
        .unwrap();

        assert_eq!(bp, UsdPrice::new_int(47_000));
    }

    #[test]
    fn bankruptcy_price_short_single_position() {
        // Short 1 BTC @ $50k, margin $3k, oracle $54k.
        // Equity = 3k + (-1)*(54k-50k) = 3k - 4k = -1k.
        // bp = 54k + (-1k)/1 = 53k.
        let user_state = UserState {
            margin: UsdValue::new_int(3_000),
            positions: BTreeMap::from([(pair_btc(), Position {
                size: Quantity::new_int(-1),
                entry_price: UsdPrice::new_int(50_000),
                entry_funding_per_unit: FundingPerUnit::ZERO,
                conditional_order_above: None,
                conditional_order_below: None,
            })]),
            ..Default::default()
        };

        let oracle_prices = BTreeMap::from([(pair_btc(), UsdPrice::new_int(54_000))]);

        let bp = compute_bankruptcy_price(
            &user_state,
            &pair_btc(),
            Quantity::new_int(1),
            &oracle_prices,
            UsdValue::ZERO,
            UsdValue::ZERO,
        )
        .unwrap();

        assert_eq!(bp, UsdPrice::new_int(53_000));
    }

    #[test]
    fn bankruptcy_price_with_other_positions() {
        // Long 1 BTC @ $50k + Long 10 ETH @ $3k, margin $5k.
        // Oracle BTC $46k, ETH $3.2k.
        // Unrealized BTC = 1*(46k-50k) = -4k.
        // Unrealized ETH = 10*(3.2k-3k) = 2k.
        // Equity = 5k + (-4k) + 2k = 3k.
        // bp = 46k - 3k/1 = 43k.
        let user_state = UserState {
            margin: UsdValue::new_int(5_000),
            positions: BTreeMap::from([
                (pair_btc(), Position {
                    size: Quantity::new_int(1),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                }),
                (pair_eth(), Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(3_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                }),
            ]),
            ..Default::default()
        };

        let oracle_prices = BTreeMap::from([
            (pair_btc(), UsdPrice::new_int(46_000)),
            (pair_eth(), UsdPrice::new_int(3_200)),
        ]);

        let bp = compute_bankruptcy_price(
            &user_state,
            &pair_btc(),
            Quantity::new_int(1),
            &oracle_prices,
            UsdValue::ZERO,
            UsdValue::ZERO,
        )
        .unwrap();

        assert_eq!(bp, UsdPrice::new_int(43_000));
    }
}
