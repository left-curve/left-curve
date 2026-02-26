use {
    dango_types::{
        Quantity, UsdPrice, UsdValue,
        perps::{PairId, PairParam, UserState},
    },
    std::collections::BTreeMap,
};

pub struct CloseEntry {
    pub pair_id: PairId,
    pub close_size: Quantity,
}

/// A policy for selecting which position(s) to close during liquidation.
/// We start from the position that contributes the most to maintenance margin
/// and go down, until the maintenance margin deficit is covered.
pub fn compute_close_schedule(
    user_state: &UserState,
    pair_params: &BTreeMap<PairId, PairParam>,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    deficit: UsdValue,
) -> anyhow::Result<Vec<CloseEntry>> {
    let mut deficit = deficit;

    // Build (mm_contribution, pair_id) list, sorted descending by MM.
    let mut mm_entries: Vec<(UsdValue, PairId)> = Vec::new();

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
    mm_entries.sort_by(|a, b| b.0.cmp(&a.0));

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
        let denom = oracle_price.checked_mul(pair_param.maintenance_margin_ratio)?;
        let close_amount = deficit.checked_div(denom)?.min(abs_size);

        // close_size = -sign(size) × close_amount (opposite direction to close)
        let close_size = if position.size.is_positive() {
            close_amount.checked_neg()?
        } else {
            close_amount
        };

        let mm_to_remove = close_amount
            .checked_mul(oracle_price)?
            .checked_mul(pair_param.maintenance_margin_ratio)?;

        deficit = deficit.checked_sub(mm_to_remove)?.max(UsdValue::ZERO);

        if close_size.is_non_zero() {
            schedule.push(CloseEntry {
                pair_id: pair_id.clone(),
                close_size,
            });
        }

        // If the full position is being closed, deficit should be exactly cleared for
        // that contribution. But if we're doing partial, deficit might still be > 0
        // only if there are precision issues. The guard at the top handles this.
    }

    Ok(schedule)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue,
            perps::{PairParam, Position},
        },
        grug::btree_map,
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
        assert_eq!(schedule[0].pair_id, pair_btc());
        // Closing a long → negative close_size
        assert_eq!(schedule[0].close_size, Quantity::new_int(-10));
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
                },
                pair_eth() => Position {
                    size: Quantity::new_int(10),
                    entry_price: UsdPrice::new_int(3_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
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
        assert_eq!(schedule[0].pair_id, pair_btc());
        assert_eq!(schedule[0].close_size, Quantity::new_int(-1));
        assert_eq!(schedule[1].pair_id, pair_eth());
        assert_eq!(schedule[1].close_size, Quantity::new_int(-10));
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
        assert_eq!(schedule[0].pair_id, pair_btc());
        // Only 2 of 10 BTC closed
        assert_eq!(schedule[0].close_size, Quantity::new_int(-2));
    }
}
