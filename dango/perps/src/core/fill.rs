use {
    dango_types::{
        Quantity, UsdPrice, UsdValue,
        perps::{PairId, PairState, Position, UserState},
    },
    grug::MathResult,
};

/// Execute a fill for a single user. Updates position and OI; settles
/// funding on the existing position.
///
/// The caller must call `accrue_funding` before invoking this function so
/// that `pair_state.funding_per_unit` is current.
///
/// Returns the raw PnL (funding + realized) as a `UsdValue`.
/// Positive = user gains, negative = user loses.
/// Does NOT include trading fees — the caller handles those separately.
pub fn execute_fill(
    pair_state: &mut PairState,
    user_state: &mut UserState,
    pair_id: &PairId,
    fill_price: UsdPrice,
    closing_size: Quantity,
    opening_size: Quantity,
) -> MathResult<UsdValue> {
    let mut total_pnl = UsdValue::ZERO;

    // Settle funding on the existing position (if any).
    if let Some(position) = user_state.positions.get_mut(pair_id) {
        let funding_pnl = settle_funding(position, pair_state)?;
        total_pnl = total_pnl.checked_add(funding_pnl)?;
    }

    // Execute the closing portion — realize PnL.
    if closing_size.is_non_zero() {
        let closing_pnl = apply_closing(user_state, pair_id, closing_size, fill_price)?;
        total_pnl = total_pnl.checked_add(closing_pnl)?;
    }

    // Execute the opening portion — grow or create position.
    if opening_size.is_non_zero() {
        apply_opening(user_state, pair_state, pair_id, opening_size, fill_price)?;
    }

    // Update open interest.
    update_oi(pair_state, closing_size, opening_size)?;

    Ok(total_pnl)
}

/// Settle funding accrued on a position since it was last touched.
///
/// Resets the position's funding entry point to the current cumulative value.
/// Returns the PnL from the user's perspective (negated accrued funding,
/// since positive accrued = user cost).
fn settle_funding(position: &mut Position, pair_state: &PairState) -> MathResult<UsdValue> {
    let accrued = crate::core::compute_position_unrealized_funding(position, pair_state)?;

    position.entry_funding_per_unit = pair_state.funding_per_unit;

    accrued.checked_neg()
}

/// Close a portion of an existing position: realize PnL and reduce size.
///
/// Removes the position entirely if fully closed.
fn apply_closing(
    user_state: &mut UserState,
    pair_id: &PairId,
    closing_size: Quantity,
    fill_price: UsdPrice,
) -> MathResult<UsdValue> {
    let position = user_state.positions.get_mut(pair_id).unwrap();

    let pnl = compute_pnl_to_realize(position, closing_size, fill_price)?;

    position.size.checked_add_assign(closing_size)?;

    if position.size.is_zero() {
        user_state.positions.remove(pair_id);
    }

    Ok(pnl)
}

/// Grow an existing position or create a new one.
///
/// For existing positions, blends the entry price as a weighted average.
/// For new positions (or positions fully closed then reopened), sets
/// the entry price and funding entry point directly.
fn apply_opening(
    user_state: &mut UserState,
    pair_state: &PairState,
    pair_id: &PairId,
    opening_size: Quantity,
    fill_price: UsdPrice,
) -> MathResult<()> {
    if let Some(position) = user_state.positions.get_mut(pair_id) {
        let old_size = position.size;
        position.size.checked_add_assign(opening_size)?;

        if old_size.is_zero() {
            // Fully closed by `apply_closing`, now reopening opposite side.
            position.entry_price = fill_price;
            position.entry_funding_per_unit = pair_state.funding_per_unit;
        } else {
            // Weighted average entry price.
            let old_notional = old_size.checked_abs()?.checked_mul(position.entry_price)?;
            let new_notional = opening_size.checked_abs()?.checked_mul(fill_price)?;

            position.entry_price = old_notional
                .checked_add(new_notional)?
                .checked_div(position.size.checked_abs()?)?;
        }
    } else {
        user_state.positions.insert(pair_id.clone(), Position {
            size: opening_size,
            entry_price: fill_price,
            entry_funding_per_unit: pair_state.funding_per_unit,
        });
    }

    Ok(())
}

/// Update open interest after a fill.
fn update_oi(
    pair_state: &mut PairState,
    closing_size: Quantity,
    opening_size: Quantity,
) -> MathResult<()> {
    if closing_size.is_negative() {
        // Closing a long position with a sell order.
        pair_state.long_oi.checked_add_assign(closing_size)?;
    } else if closing_size.is_positive() {
        // Closing a short position with a buy order.
        pair_state.short_oi.checked_sub_assign(closing_size)?;
    }

    if opening_size.is_positive() {
        // Opening a long position with a buy order.
        pair_state.long_oi.checked_add_assign(opening_size)?;
    } else if opening_size.is_negative() {
        // Opening a short position with a sell order.
        pair_state
            .short_oi
            .checked_add_assign(opening_size.checked_abs()?)?;
    }

    Ok(())
}

/// Compute the PnL to be realized when closing a portion of a position.
///
/// - Long positions: profit when exit > entry
/// - Short positions: profit when entry > exit
fn compute_pnl_to_realize(
    position: &Position,
    closing_size: Quantity,
    fill_price: UsdPrice,
) -> MathResult<UsdValue> {
    let entry_value = closing_size
        .checked_abs()?
        .checked_mul(position.entry_price)?;
    let exit_value = closing_size.checked_abs()?.checked_mul(fill_price)?;

    if position.size.is_positive() {
        Ok(exit_value.checked_sub(entry_value)?)
    } else {
        Ok(entry_value.checked_sub(exit_value)?)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{FundingPerUnit, perps::PairState},
        grug::Timestamp,
        std::collections::BTreeMap,
    };

    fn pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn default_pair_state() -> PairState {
        PairState::new(Timestamp::from_nanos(0))
    }

    fn make_user_state(size: i128, entry_price: i128) -> UserState {
        let mut positions = BTreeMap::new();
        positions.insert(pair_id(), Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(entry_price),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        UserState {
            positions,
            ..Default::default()
        }
    }

    // ---- execute_fill: open new position ----

    #[test]
    fn open_new_long() {
        let mut pair_state = default_pair_state();
        let mut user_state = UserState::default();

        let pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::ZERO,
            Quantity::new_int(10),
        )
        .unwrap();

        // No PnL on opening.
        assert_eq!(pnl, UsdValue::ZERO);

        // Position created.
        let pos = user_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_000));

        // OI updated.
        assert_eq!(pair_state.long_oi, Quantity::new_int(10));
        assert_eq!(pair_state.short_oi, Quantity::ZERO);
    }

    #[test]
    fn open_new_short() {
        let mut pair_state = default_pair_state();
        let mut user_state = UserState::default();

        let pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::ZERO,
            Quantity::new_int(-10),
        )
        .unwrap();

        assert_eq!(pnl, UsdValue::ZERO);

        let pos = user_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(-10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_000));

        assert_eq!(pair_state.long_oi, Quantity::ZERO);
        assert_eq!(pair_state.short_oi, Quantity::new_int(10));
    }

    // ---- execute_fill: close position ----

    #[test]
    fn close_long_at_profit() {
        let mut pair_state = default_pair_state();
        pair_state.long_oi = Quantity::new_int(10);
        let mut user_state = make_user_state(10, 50_000);

        let pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(55_000),
            Quantity::new_int(-10), // closing a long with a sell
            Quantity::ZERO,
        )
        .unwrap();

        // PnL = 10 * (55000 - 50000) = 50000 USD
        assert_eq!(pnl, UsdValue::new_int(50_000));

        // Position removed.
        assert!(!user_state.positions.contains_key(&pair_id()));

        // OI reduced.
        assert_eq!(pair_state.long_oi, Quantity::ZERO);
    }

    #[test]
    fn close_long_at_loss() {
        let mut pair_state = default_pair_state();
        pair_state.long_oi = Quantity::new_int(10);
        let mut user_state = make_user_state(10, 50_000);

        let pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(48_000),
            Quantity::new_int(-10),
            Quantity::ZERO,
        )
        .unwrap();

        // PnL = 10 * (48000 - 50000) = -20000 USD
        assert_eq!(pnl, UsdValue::new_int(-20_000));
    }

    #[test]
    fn close_short_at_profit() {
        let mut pair_state = default_pair_state();
        pair_state.short_oi = Quantity::new_int(10);
        let mut user_state = make_user_state(-10, 50_000);

        let pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(48_000),
            Quantity::new_int(10), // closing a short with a buy
            Quantity::ZERO,
        )
        .unwrap();

        // PnL = 10 * (50000 - 48000) = 20000 USD
        assert_eq!(pnl, UsdValue::new_int(20_000));
    }

    // ---- execute_fill: partial close ----

    #[test]
    fn partial_close_keeps_position() {
        let mut pair_state = default_pair_state();
        pair_state.long_oi = Quantity::new_int(10);
        let mut user_state = make_user_state(10, 50_000);

        let _pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(55_000),
            Quantity::new_int(-4), // close 4 of 10
            Quantity::ZERO,
        )
        .unwrap();

        let pos = user_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(6));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_000)); // unchanged
        assert_eq!(pair_state.long_oi, Quantity::new_int(6));
    }

    // ---- execute_fill: flip direction ----

    #[test]
    fn flip_long_to_short() {
        let mut pair_state = default_pair_state();
        pair_state.long_oi = Quantity::new_int(5);
        let mut user_state = make_user_state(5, 50_000);

        let _pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(52_000),
            Quantity::new_int(-5), // close the long
            Quantity::new_int(-3), // open a short
        )
        .unwrap();

        let pos = user_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(-3));
        assert_eq!(pos.entry_price, UsdPrice::new_int(52_000));

        assert_eq!(pair_state.long_oi, Quantity::ZERO);
        assert_eq!(pair_state.short_oi, Quantity::new_int(3));
    }

    // ---- execute_fill: weighted average entry price ----

    #[test]
    fn increase_long_blends_entry_price() {
        let mut pair_state = default_pair_state();
        pair_state.long_oi = Quantity::new_int(10);
        let mut user_state = make_user_state(10, 50_000);

        let pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(60_000),
            Quantity::ZERO,
            Quantity::new_int(10), // double the position
        )
        .unwrap();

        assert_eq!(pnl, UsdValue::ZERO);

        let pos = user_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(20));
        // Weighted avg: (10*50000 + 10*60000) / 20 = 55000
        assert_eq!(pos.entry_price, UsdPrice::new_int(55_000));

        assert_eq!(pair_state.long_oi, Quantity::new_int(20));
    }

    // ---- execute_fill: funding settlement ----

    #[test]
    fn funding_settled_on_fill() {
        let mut pair_state = default_pair_state();
        // Simulate accumulated funding of 100 USD per unit.
        pair_state.funding_per_unit = FundingPerUnit::new_int(100);
        pair_state.long_oi = Quantity::new_int(10);

        let mut positions = BTreeMap::new();
        positions.insert(pair_id(), Position {
            size: Quantity::new_int(10),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO, // entered at 0
        });
        let mut user_state = UserState {
            positions,
            ..Default::default()
        };

        // Open more — the funding should be settled first.
        let pnl = execute_fill(
            &mut pair_state,
            &mut user_state,
            &pair_id(),
            UsdPrice::new_int(50_000),
            Quantity::ZERO,
            Quantity::new_int(5),
        )
        .unwrap();

        // Funding accrued = 10 * (100 - 0) = 1000 USD. User pays (longs pay when funding positive).
        // settle_funding returns negated accrued = -1000 USD as PnL.
        assert_eq!(pnl, UsdValue::new_int(-1000));

        // Funding entry point reset.
        let pos = user_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.entry_funding_per_unit, FundingPerUnit::new_int(100));
    }
}
