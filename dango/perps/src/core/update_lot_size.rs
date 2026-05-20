//! Re-align resting orders and open positions on a trading pair to a new
//! `lot_size` precision grid.
//!
//! Intended call sites: contract migration (one-shot upgrade) and any
//! future `lot_size` change via `maintain::configure`. Kept in `core` so
//! the algorithm has a single home regardless of which entry point
//! triggers it.
//!
//! Three passes per pair:
//!
//! 1. **Orders.** Every resting order's `size` is truncated toward zero to
//!    a `new_lot_size` multiple. Sub-lot orders (rounding to zero) are
//!    cancelled with a full margin refund; otherwise the order is shrunk
//!    in place and `reserved_margin` is scaled proportionally so the
//!    `IM == sum(open-orders.reserved_margin)` invariant holds.
//!
//! 2. **Positions.** Every open position's `size` is truncated the same
//!    way. Positions that round to zero are removed (along with their
//!    `LONGS`/`SHORTS` index entry). `entry_price` is preserved so the
//!    index key for non-zero positions is unchanged. `margin` is **not**
//!    touched — truncation effectively closes the trimmed lots at the
//!    entry price, realizing zero PnL. Conditional orders attached to a
//!    surviving position (`conditional_order_above` / `_below`) have
//!    their partial-close `size` aligned too; a sub-lot partial close
//!    is dropped to avoid widening the trigger into a full close.
//!
//! 3. **OI rebalance.** After truncation, both `long_oi` and `short_oi`
//!    are sums of lot-aligned positions, so their difference is itself a
//!    multiple of `lot_size`. To restore the closure invariant
//!    `long_oi == short_oi`, one lot at a time is trimmed off positions
//!    on the heavier side (deterministic walk over `USER_STATES`) until
//!    the imbalance is consumed.

use {
    crate::state::{LONGS, PAIR_STATES, SHORTS, USER_STATES},
    dango_order_book::{
        ASKS, BIDS, LimitOrder, OrderId, OrderKey, PairId, Quantity, ReasonForOrderRemoval,
        UsdPrice, may_invert_price, remove_order,
    },
    dango_types::perps::UserState,
    grug::{Addr, EventBuilder, MathResult, Order as IterationOrder, StdResult, Storage},
    std::collections::BTreeSet,
};

/// Truncate `size` toward zero to the nearest multiple of `lot_size`.
///
/// Distinct from [`Quantity::checked_floor_multiple`], which floors toward
/// negative infinity. For a negative `size` (a short position), floor-toward-
/// negative-infinity would *grow* `|size|`; floor-toward-zero shrinks it,
/// which is what we want when bringing an existing value down onto the
/// `lot_size` grid.
fn truncate_to_lot(size: Quantity, lot_size: Quantity) -> MathResult<Quantity> {
    let aligned_abs = size.checked_abs()?.checked_floor_multiple(lot_size)?;
    if size.is_negative() {
        aligned_abs.checked_neg()
    } else {
        Ok(aligned_abs)
    }
}

/// Whether the lot-size change requires an alignment pass.
///
/// Rule: alignment is needed only when the new grid step does not
/// divide the old one — i.e., values aligned to `old_lot_size` are not
/// automatically aligned to `new_lot_size`. Concretely:
/// - `old = 10, new = 5` → skip. Every multiple of 10 is also a
///   multiple of 5.
/// - `old = 5, new = 10` → align. A position of 5 is not a multiple
///   of 10.
/// - `old = 10, new = 3` → align. 10 is not a multiple of 3, so
///   existing 10-aligned positions need re-truncation.
///
/// Special cases:
/// - `new_lot_size == 0` — the lot constraint is disabled; no
///   enforcement, nothing to align to.
/// - `old_lot_size == 0` — initial bring-up; the new value imposes a
///   fresh grid on previously-unconstrained state, so alignment is
///   needed.
fn is_alignment_needed(old_lot_size: Quantity, new_lot_size: Quantity) -> MathResult<bool> {
    if new_lot_size.is_zero() {
        return Ok(false);
    }
    if old_lot_size.is_zero() {
        return Ok(true);
    }
    Ok(!old_lot_size.checked_rem(new_lot_size)?.is_zero())
}

/// Re-align all resting orders and open positions on `pair_id` to the
/// `new_lot_size` grid, restoring the `long_oi == short_oi` closure
/// invariant if it drifts during truncation.
///
/// Idempotent: a second invocation with the same `new_lot_size` is a
/// no-op (`is_alignment_needed` short-circuits when `new` divides
/// `old`, which includes the `old == new` case).
pub fn update_lot_size(
    storage: &mut dyn Storage,
    pair_id: &PairId,
    old_lot_size: Quantity,
    new_lot_size: Quantity,
    bucket_sizes: &BTreeSet<UsdPrice>,
    events: Option<&mut EventBuilder>,
) -> anyhow::Result<()> {
    if !is_alignment_needed(old_lot_size, new_lot_size)? {
        return Ok(());
    }

    align_resting_orders(storage, pair_id, new_lot_size, bucket_sizes, events)?;

    align_positions(storage, pair_id, new_lot_size)?;

    rebalance_oi(storage, pair_id, new_lot_size)?;

    Ok(())
}

// --------------------------- pass 1: orders ----------------------------------

fn align_resting_orders(
    storage: &mut dyn Storage,
    pair_id: &PairId,
    new_lot_size: Quantity,
    bucket_sizes: &BTreeSet<UsdPrice>,
    mut events: Option<&mut EventBuilder>,
) -> anyhow::Result<()> {
    // Materialize before mutating: tx-level iterators see snapshot
    // semantics, but storage writes during iteration can still surprise
    // the lower-level KV store. `cancel_all_orders` uses the same
    // collect-then-mutate pattern.
    let bids: Vec<((UsdPrice, OrderId), LimitOrder)> = BIDS
        .prefix(pair_id.clone())
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    let asks: Vec<((UsdPrice, OrderId), LimitOrder)> = ASKS
        .prefix(pair_id.clone())
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for ((stored_price, order_id), order) in bids {
        align_one_order(
            storage,
            pair_id,
            stored_price,
            order_id,
            order,
            // is_bid
            true,
            new_lot_size,
            bucket_sizes,
            events.as_deref_mut(),
        )?;
    }

    for ((stored_price, order_id), order) in asks {
        align_one_order(
            storage,
            pair_id,
            stored_price,
            order_id,
            order,
            // is_bid
            false,
            new_lot_size,
            bucket_sizes,
            events.as_deref_mut(),
        )?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn align_one_order(
    storage: &mut dyn Storage,
    pair_id: &PairId,
    stored_price: UsdPrice,
    order_id: OrderId,
    order: LimitOrder,
    is_bid: bool,
    new_lot_size: Quantity,
    bucket_sizes: &BTreeSet<UsdPrice>,
    events: Option<&mut EventBuilder>,
) -> anyhow::Result<()> {
    let new_size = truncate_to_lot(order.size, new_lot_size)?;

    if new_size == order.size {
        return Ok(());
    }

    let order_key: OrderKey = (pair_id.clone(), stored_price, order_id);

    if new_size.is_zero() {
        cancel_sub_lot_order(storage, order_key, &order, bucket_sizes, events)?;
    } else {
        shrink_order(
            storage,
            order_key,
            order,
            new_size,
            is_bid,
            pair_id,
            bucket_sizes,
        )?;
    }

    Ok(())
}

/// Cancel an order whose truncated size would be sub-lot. Full margin
/// refund: there is no lot-aligned residual to keep on the book, so the
/// user receives back the entire `reserved_margin` and their
/// `open_order_count` decrements.
fn cancel_sub_lot_order(
    storage: &mut dyn Storage,
    order_key: OrderKey,
    order: &LimitOrder,
    bucket_sizes: &BTreeSet<UsdPrice>,
    events: Option<&mut EventBuilder>,
) -> anyhow::Result<()> {
    let mut user_state = USER_STATES.load(storage, order.user)?;
    (user_state.reserved_margin).checked_sub_assign(order.reserved_margin)?;
    user_state.open_order_count = user_state.open_order_count.saturating_sub(1);

    remove_order(
        storage,
        order_key,
        order,
        ReasonForOrderRemoval::Canceled,
        bucket_sizes,
        events,
    )?;

    USER_STATES.save(storage, order.user, &user_state)?;
    Ok(())
}

/// Shrink an order in place to `new_size`. The order key is unchanged
/// (price and order id don't move), so we save back onto the same slot.
///
/// `reserved_margin` is scaled by `|new| / |old|` so the user's locked
/// margin matches the smaller order. The freed slice returns to
/// `user_state.reserved_margin`.
///
/// Depth bookkeeping decrements by the size delta only (the order is
/// still on the book).
fn shrink_order(
    storage: &mut dyn Storage,
    order_key: OrderKey,
    order: LimitOrder,
    new_size: Quantity,
    is_bid: bool,
    pair_id: &PairId,
    bucket_sizes: &BTreeSet<UsdPrice>,
) -> anyhow::Result<()> {
    let old_abs = order.size.checked_abs()?;
    let new_abs = new_size.checked_abs()?;
    let delta_abs = old_abs.checked_sub(new_abs)?;

    // |new| < |old| by construction (`truncate_to_lot` only ever shrinks);
    // the ratio is in (0, 1) and `checked_mul` rounds down, which keeps
    // the refund ≥ 0 and the new reserved margin ≤ the old one.
    let scale = new_abs.checked_div(old_abs)?;
    let new_reserved = order.reserved_margin.checked_mul(scale)?;
    let freed = order.reserved_margin.checked_sub(new_reserved)?;

    // Release margin from the user.
    let mut user_state = USER_STATES.load(storage, order.user)?;
    (user_state.reserved_margin).checked_sub_assign(freed)?;
    USER_STATES.save(storage, order.user, &user_state)?;

    // Decrement depth by the shrunk slice.
    let real_price = may_invert_price(order_key.1, is_bid);
    dango_order_book::decrease_liquidity_depths(
        storage,
        pair_id,
        is_bid,
        real_price,
        delta_abs,
        bucket_sizes,
    )?;

    // Write the order back at the same key with the new size and
    // proportional reserved margin.
    let new_order = LimitOrder {
        size: new_size,
        reserved_margin: new_reserved,
        ..order
    };
    if is_bid {
        BIDS.save(storage, order_key, &new_order)?;
    } else {
        ASKS.save(storage, order_key, &new_order)?;
    }

    Ok(())
}

// --------------------------- pass 2: positions -------------------------------

fn align_positions(
    storage: &mut dyn Storage,
    pair_id: &PairId,
    new_lot_size: Quantity,
) -> anyhow::Result<()> {
    let users: Vec<(Addr, UserState)> = USER_STATES
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let mut pair_state = PAIR_STATES.load(storage, pair_id)?;

    for (user_addr, mut user_state) in users {
        let Some(position) = user_state.positions.get(pair_id) else {
            continue;
        };
        let old_size = position.size;
        let entry_price = position.entry_price;
        let new_size = truncate_to_lot(old_size, new_lot_size)?;

        let mut changed = false;

        if new_size != old_size {
            let delta_abs = old_size
                .checked_abs()?
                .checked_sub(new_size.checked_abs()?)?;
            if old_size.is_positive() {
                (pair_state.long_oi).checked_sub_assign(delta_abs)?;
            } else {
                (pair_state.short_oi).checked_sub_assign(delta_abs)?;
            }

            if new_size.is_zero() {
                user_state.positions.remove(pair_id);
                if old_size.is_positive() {
                    LONGS.remove(storage, (pair_id.clone(), entry_price, user_addr));
                } else {
                    SHORTS.remove(storage, (pair_id.clone(), entry_price, user_addr));
                }
            } else {
                // entry_price unchanged → LONGS/SHORTS key unchanged →
                // no reindex.
                user_state.positions.get_mut(pair_id).unwrap().size = new_size;
            }
            changed = true;
        }

        // Conditional orders on the surviving position. A TP/SL with a
        // sub-lot partial-close size would otherwise fire and submit an
        // unaligned market order at trigger time.
        if let Some(position) = user_state.positions.get_mut(pair_id) {
            if align_conditional_order_size(&mut position.conditional_order_above, new_lot_size)? {
                changed = true;
            }
            if align_conditional_order_size(&mut position.conditional_order_below, new_lot_size)? {
                changed = true;
            }
        }

        if changed {
            // Re-saving through the indexed map reindexes
            // `conditional_orders` so any dropped trigger no longer
            // shows up in cron-driven iteration.
            USER_STATES.save(storage, user_addr, &user_state)?;
        }
    }

    PAIR_STATES.save(storage, pair_id, &pair_state)?;
    Ok(())
}

/// Align a conditional order's partial-close size to the new lot grid.
/// `None` means "close the entire position at trigger" — already grid-
/// agnostic since the position itself is lot-aligned by then. A
/// `Some(s)` with sub-lot magnitude is dropped entirely (slot set to
/// `None`) rather than coerced to a full close: silently widening the
/// trigger from "close N units" to "close everything" would be a
/// surprising change of intent.
///
/// Returns `true` iff the slot was modified.
fn align_conditional_order_size(
    slot: &mut Option<dango_order_book::ConditionalOrder>,
    new_lot_size: Quantity,
) -> MathResult<bool> {
    let Some(co) = slot.as_ref() else {
        return Ok(false);
    };
    let Some(co_size) = co.size else {
        return Ok(false);
    };
    let new_co_size = truncate_to_lot(co_size, new_lot_size)?;
    if new_co_size == co_size {
        return Ok(false);
    }
    if new_co_size.is_zero() {
        *slot = None;
    } else {
        slot.as_mut().unwrap().size = Some(new_co_size);
    }
    Ok(true)
}

// --------------------------- pass 3: OI rebalance ----------------------------

fn rebalance_oi(
    storage: &mut dyn Storage,
    pair_id: &PairId,
    new_lot_size: Quantity,
) -> anyhow::Result<()> {
    let mut pair_state = PAIR_STATES.load(storage, pair_id)?;
    if pair_state.long_oi == pair_state.short_oi {
        return Ok(());
    }

    let (mut remaining, heavier_is_long) = if pair_state.long_oi > pair_state.short_oi {
        ((pair_state.long_oi).checked_sub(pair_state.short_oi)?, true)
    } else {
        (
            (pair_state.short_oi).checked_sub(pair_state.long_oi)?,
            false,
        )
    };

    // Round-robin: take one lot from each eligible user per pass and
    // repeat until the imbalance is consumed. Spreads the cost across
    // users — a single user loses at most `ceil(imbalance / eligible_users)`
    // lots, never the whole imbalance.
    //
    // Per-pass we re-collect `USER_STATES` so positions removed by a
    // prior pass are dropped from the working set, which guarantees
    // forward progress.
    loop {
        if remaining.is_zero() {
            break;
        }

        let users = USER_STATES
            .range(storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        let mut trimmed_this_pass = Quantity::ZERO;

        for (user_addr, mut user_state) in users {
            if remaining.is_zero() {
                break;
            }
            let Some(position) = user_state.positions.get(pair_id) else {
                continue;
            };
            let on_heavier_side = if heavier_is_long {
                position.size.is_positive()
            } else {
                position.size.is_negative()
            };
            if !on_heavier_side {
                continue;
            }

            let pos_abs = position.size.checked_abs()?;
            let entry_price = position.entry_price;
            let new_abs = pos_abs.checked_sub(new_lot_size)?;
            let new_size = if heavier_is_long {
                new_abs
            } else {
                new_abs.checked_neg()?
            };

            if new_abs.is_zero() {
                user_state.positions.remove(pair_id);
                if heavier_is_long {
                    LONGS.remove(storage, (pair_id.clone(), entry_price, user_addr));
                } else {
                    SHORTS.remove(storage, (pair_id.clone(), entry_price, user_addr));
                }
            } else {
                user_state.positions.get_mut(pair_id).unwrap().size = new_size;
            }

            USER_STATES.save(storage, user_addr, &user_state)?;

            if heavier_is_long {
                (pair_state.long_oi).checked_sub_assign(new_lot_size)?;
            } else {
                (pair_state.short_oi).checked_sub_assign(new_lot_size)?;
            }
            (remaining).checked_sub_assign(new_lot_size)?;
            (trimmed_this_pass).checked_add_assign(new_lot_size)?;
        }

        // Loop-safety: if no eligible position was found across an
        // entire pass, the heavier side has no more positions to trim
        // but `remaining > 0`. That means the OI tally diverged from
        // the actual sum of position sizes — a hard invariant violation
        // upstream, not something we can correct here.
        anyhow::ensure!(
            !trimmed_this_pass.is_zero(),
            "oi rebalance failed for pair {pair_id}: {remaining} of imbalance could not be absorbed"
        );
    }

    PAIR_STATES.save(storage, pair_id, &pair_state)?;
    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::state::USER_STATES,
        dango_order_book::{ConditionalOrder, Dimensionless, FundingPerUnit, LimitOrder, UsdValue},
        dango_types::perps::{PairState, Position, UserState},
        grug::{MockStorage, Timestamp, Uint64},
        std::collections::{BTreeMap, BTreeSet},
    };

    fn pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn addr(byte: u8) -> Addr {
        Addr::mock(byte)
    }

    fn entry_price() -> UsdPrice {
        UsdPrice::new_int(50_000)
    }

    fn save_position(storage: &mut dyn Storage, who: Addr, size: i128) {
        let mut positions = BTreeMap::new();
        positions.insert(pair_id(), Position {
            size: Quantity::new_int(size),
            entry_price: entry_price(),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        });
        USER_STATES
            .save(storage, who, &UserState {
                positions,
                ..Default::default()
            })
            .unwrap();
        // Mirror the LONGS/SHORTS index that the contract maintains.
        if size > 0 {
            LONGS
                .insert(storage, (pair_id(), entry_price(), who))
                .unwrap();
        } else if size < 0 {
            SHORTS
                .insert(storage, (pair_id(), entry_price(), who))
                .unwrap();
        }
    }

    fn save_pair_state(storage: &mut dyn Storage, long_oi: i128, short_oi: i128) {
        PAIR_STATES
            .save(storage, &pair_id(), &PairState {
                long_oi: Quantity::new_int(long_oi),
                short_oi: Quantity::new_int(short_oi),
                ..Default::default()
            })
            .unwrap();
    }

    fn load_position_size(storage: &dyn Storage, who: Addr) -> Option<Quantity> {
        USER_STATES
            .may_load(storage, who)
            .unwrap()
            .and_then(|us| us.positions.get(&pair_id()).map(|p| p.size))
    }

    // ---- truncate_to_lot ----

    #[test]
    fn truncate_toward_zero_positive() {
        let r = truncate_to_lot(Quantity::new_int(7), Quantity::new_int(5)).unwrap();
        assert_eq!(r, Quantity::new_int(5));
    }

    #[test]
    fn truncate_toward_zero_negative() {
        // Contrast with `checked_floor_multiple` (floor toward -∞) which
        // would return -10. We want toward-zero: -5.
        let r = truncate_to_lot(Quantity::new_int(-7), Quantity::new_int(5)).unwrap();
        assert_eq!(r, Quantity::new_int(-5));
    }

    #[test]
    fn truncate_exact_multiple_unchanged() {
        let r = truncate_to_lot(Quantity::new_int(10), Quantity::new_int(5)).unwrap();
        assert_eq!(r, Quantity::new_int(10));
    }

    #[test]
    fn truncate_sub_lot_to_zero() {
        let r = truncate_to_lot(Quantity::new_int(3), Quantity::new_int(5)).unwrap();
        assert_eq!(r, Quantity::ZERO);
    }

    #[test]
    fn truncate_sub_lot_negative_to_zero() {
        // Same as the positive case — `|-3| < 5` floors to 0, and 0
        // has no sign to preserve.
        let r = truncate_to_lot(Quantity::new_int(-3), Quantity::new_int(5)).unwrap();
        assert_eq!(r, Quantity::ZERO);
    }

    // ---- alignment-needed predicate ----

    #[test]
    fn alignment_not_needed_when_new_is_zero() {
        // Disabling the constraint — nothing to enforce.
        assert!(!is_alignment_needed(Quantity::new_int(5), Quantity::ZERO).unwrap());
    }

    #[test]
    fn alignment_not_needed_when_new_divides_old() {
        // 10 -> 5: every multiple of 10 is also a multiple of 5, so
        // existing aligned state carries over.
        assert!(!is_alignment_needed(Quantity::new_int(10), Quantity::new_int(5)).unwrap());
    }

    #[test]
    fn alignment_needed_when_old_zero_and_new_positive() {
        // Initial bring-up: old was disabled, new sets an actual grid.
        assert!(is_alignment_needed(Quantity::ZERO, Quantity::new_int(5)).unwrap());
    }

    #[test]
    fn alignment_needed_when_going_coarser() {
        // 5 -> 10: a position of 5 is not a multiple of 10.
        assert!(is_alignment_needed(Quantity::new_int(5), Quantity::new_int(10)).unwrap());
    }

    #[test]
    fn alignment_needed_when_new_doesnt_divide_old() {
        // 10 -> 3: 10 is not a multiple of 3, so 10-aligned positions
        // need re-truncation even though new < old.
        assert!(is_alignment_needed(Quantity::new_int(10), Quantity::new_int(3)).unwrap());
    }

    // ---- positions: per-side truncation ----

    /// 7 long → 5 (one lot trimmed off). OI decrements by 2 on the long side.
    #[test]
    fn align_positions_truncates_long_and_decrements_oi() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), 7);
        save_pair_state(&mut storage, 7, 0);

        align_positions(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        assert_eq!(
            load_position_size(&storage, addr(1)),
            Some(Quantity::new_int(5))
        );
        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::new_int(5));
        assert_eq!(pair_state.short_oi, Quantity::ZERO);
    }

    #[test]
    fn align_positions_truncates_short() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), -7);
        save_pair_state(&mut storage, 0, 7);

        align_positions(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        assert_eq!(
            load_position_size(&storage, addr(1)),
            Some(Quantity::new_int(-5))
        );
        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::ZERO);
        assert_eq!(pair_state.short_oi, Quantity::new_int(5));
    }

    /// Build a `ConditionalOrder` with a partial-close size of `size`.
    /// `order_id` is encoded into the trigger price so two test orders
    /// can sit under the same `conditional_orders` multi-index without
    /// colliding.
    fn conditional_order(size: Option<i128>, order_id: u64) -> ConditionalOrder {
        ConditionalOrder {
            order_id: Uint64::new(order_id),
            size: size.map(Quantity::new_int),
            trigger_price: UsdPrice::new_int(50_000 + order_id as i128),
            max_slippage: Dimensionless::ZERO,
        }
    }

    fn attach_conditional_above(storage: &mut dyn Storage, who: Addr, co: ConditionalOrder) {
        let mut user_state = USER_STATES.load(storage, who).unwrap();
        user_state
            .positions
            .get_mut(&pair_id())
            .unwrap()
            .conditional_order_above = Some(co);
        USER_STATES.save(storage, who, &user_state).unwrap();
    }

    fn attach_conditional_below(storage: &mut dyn Storage, who: Addr, co: ConditionalOrder) {
        let mut user_state = USER_STATES.load(storage, who).unwrap();
        user_state
            .positions
            .get_mut(&pair_id())
            .unwrap()
            .conditional_order_below = Some(co);
        USER_STATES.save(storage, who, &user_state).unwrap();
    }

    fn load_conditional_above(storage: &dyn Storage, who: Addr) -> Option<ConditionalOrder> {
        USER_STATES.may_load(storage, who).unwrap().and_then(|us| {
            us.positions
                .get(&pair_id())
                .and_then(|p| p.conditional_order_above.clone())
        })
    }

    fn load_conditional_below(storage: &dyn Storage, who: Addr) -> Option<ConditionalOrder> {
        USER_STATES.may_load(storage, who).unwrap().and_then(|us| {
            us.positions
                .get(&pair_id())
                .and_then(|p| p.conditional_order_below.clone())
        })
    }

    /// A position that rounds to zero is removed entirely, including its
    /// LONGS/SHORTS index entry.
    #[test]
    fn align_positions_removes_sub_lot_long() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), 3);
        save_pair_state(&mut storage, 3, 0);

        align_positions(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        assert_eq!(load_position_size(&storage, addr(1)), None);
        assert!(
            !LONGS.has(&storage, (pair_id(), entry_price(), addr(1))),
            "LONGS entry not cleaned up",
        );
        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::ZERO);
    }

    // ---- conditional orders ----

    /// Conditional order with a non-aligned partial-close size gets its
    /// `size` truncated. The order itself stays attached to the
    /// position.
    #[test]
    fn align_positions_truncates_conditional_above_size() {
        let mut storage = MockStorage::new();
        // Long of 20 with a TP-above that closes 7 lots-of-old-grid.
        // After truncating to lot_size 5, the TP size becomes 5.
        save_position(&mut storage, addr(1), 20);
        save_pair_state(&mut storage, 20, 0);
        // Conditional-above closes part of the long, so the size is
        // negative (opposes the position).
        attach_conditional_above(&mut storage, addr(1), conditional_order(Some(-7), 1));

        align_positions(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        let co = load_conditional_above(&storage, addr(1)).unwrap();
        assert_eq!(co.size, Some(Quantity::new_int(-5)));
    }

    /// Conditional order whose partial-close size truncates to zero is
    /// dropped (slot becomes `None`) rather than re-interpreted as a
    /// full close.
    #[test]
    fn align_positions_drops_sub_lot_conditional() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), 20);
        save_pair_state(&mut storage, 20, 0);
        // Partial-close of 3 (sub-lot at lot_size 5).
        attach_conditional_below(&mut storage, addr(1), conditional_order(Some(-3), 2));

        align_positions(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        assert!(load_conditional_below(&storage, addr(1)).is_none());
        // The position itself is untouched.
        assert_eq!(
            load_position_size(&storage, addr(1)),
            Some(Quantity::new_int(20))
        );
    }

    /// Conditional order with `size = None` (full close at trigger) is
    /// grid-agnostic — left untouched.
    #[test]
    fn align_positions_leaves_full_close_conditional_alone() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), 20);
        save_pair_state(&mut storage, 20, 0);
        attach_conditional_above(&mut storage, addr(1), conditional_order(None, 1));

        align_positions(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        let co = load_conditional_above(&storage, addr(1)).unwrap();
        assert_eq!(co.size, None);
    }

    /// A position whose `size` is already aligned but whose conditional
    /// order is sub-lot still triggers the conditional-order pass —
    /// we cannot `continue` on size alignment alone.
    #[test]
    fn align_positions_aligns_conditional_even_if_position_size_is_aligned() {
        let mut storage = MockStorage::new();
        // Position 20 — already a multiple of 5.
        save_position(&mut storage, addr(1), 20);
        save_pair_state(&mut storage, 20, 0);
        // But the TP has a sub-lot partial-close size.
        attach_conditional_above(&mut storage, addr(1), conditional_order(Some(-3), 1));

        align_positions(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        // Position untouched, conditional order dropped.
        assert_eq!(
            load_position_size(&storage, addr(1)),
            Some(Quantity::new_int(20))
        );
        assert!(load_conditional_above(&storage, addr(1)).is_none());
    }

    // ---- OI rebalance ----

    /// Two longs at 5 each (sum 10), one short at 7 truncated to 5 (sum 5).
    /// `align_positions` leaves long_oi=10, short_oi=5; rebalance trims one
    /// long down by 5.
    #[test]
    fn rebalance_trims_heavier_long_side() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), 5);
        save_position(&mut storage, addr(2), 5);
        save_pair_state(&mut storage, 10, 5);

        rebalance_oi(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::new_int(5));
        assert_eq!(pair_state.short_oi, Quantity::new_int(5));
        // One of the two long positions absorbed the trim; the other is
        // untouched.
        let surviving = [addr(1), addr(2)]
            .into_iter()
            .filter_map(|a| load_position_size(&storage, a).map(|s| (a, s)))
            .collect::<Vec<_>>();
        assert_eq!(surviving.len(), 1);
        assert_eq!(surviving[0].1, Quantity::new_int(5));
    }

    /// Imbalance of 2 lots, two eligible long users with positions
    /// large enough to absorb more than one lot each. Round-robin
    /// distributes the cost: each user loses exactly one lot, neither
    /// loses both.
    #[test]
    fn rebalance_spreads_cost_across_users() {
        let mut storage = MockStorage::new();
        // Two longs of 15 each (3 lots each); one short of 20
        // (4 lots). long_oi = 30, short_oi = 20, imbalance = 10
        // (= 2 lots at lot_size 5).
        save_position(&mut storage, addr(1), 15);
        save_position(&mut storage, addr(2), 15);
        save_position(&mut storage, addr(3), -20);
        save_pair_state(&mut storage, 30, 20);

        rebalance_oi(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::new_int(20));
        assert_eq!(pair_state.short_oi, Quantity::new_int(20));
        // Each long user lost exactly one lot — not two from a single
        // user.
        assert_eq!(
            load_position_size(&storage, addr(1)),
            Some(Quantity::new_int(10))
        );
        assert_eq!(
            load_position_size(&storage, addr(2)),
            Some(Quantity::new_int(10))
        );
        assert_eq!(
            load_position_size(&storage, addr(3)),
            Some(Quantity::new_int(-20))
        );
    }

    /// Imbalance larger than (eligible_users × lot_size) wraps to a
    /// second pass, taking another lot from each user.
    #[test]
    fn rebalance_wraps_to_second_pass() {
        let mut storage = MockStorage::new();
        // Two longs of 15 each (3 lots each); one short of 5 (1 lot).
        // long_oi = 30, short_oi = 5, imbalance = 25 (= 5 lots). Two
        // eligible users → first pass trims 2 lots (one each), second
        // pass trims 2 more, third pass trims 1 more from one of them.
        save_position(&mut storage, addr(1), 15);
        save_position(&mut storage, addr(2), 15);
        save_position(&mut storage, addr(3), -5);
        save_pair_state(&mut storage, 30, 5);

        rebalance_oi(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::new_int(5));
        assert_eq!(pair_state.short_oi, Quantity::new_int(5));
        // Combined long positions sum to 5 (one lot). Distribution
        // between the two longs depends on iteration order: addr(1)
        // comes first, so it takes the extra lot off last and ends up
        // empty; addr(2) keeps one lot.
        let long_sum = [addr(1), addr(2)]
            .into_iter()
            .filter_map(|a| load_position_size(&storage, a))
            .fold(Quantity::ZERO, |acc, s| acc.checked_add(s).unwrap());
        assert_eq!(long_sum, Quantity::new_int(5));
    }

    #[test]
    fn rebalance_no_op_when_already_balanced() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), 5);
        save_position(&mut storage, addr(2), -5);
        save_pair_state(&mut storage, 5, 5);

        rebalance_oi(&mut storage, &pair_id(), Quantity::new_int(5)).unwrap();

        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::new_int(5));
        assert_eq!(pair_state.short_oi, Quantity::new_int(5));
        assert_eq!(
            load_position_size(&storage, addr(1)),
            Some(Quantity::new_int(5))
        );
        assert_eq!(
            load_position_size(&storage, addr(2)),
            Some(Quantity::new_int(-5))
        );
    }

    // ---- orders: cancel sub-lot, shrink in place ----

    /// Empty `bucket_sizes` makes `decrease_liquidity_depths` a no-op so
    /// the order-path tests don't need to seed `DEPTHS`. The depth
    /// bookkeeping itself is covered by the order-book crate's own
    /// suite.
    fn empty_buckets() -> BTreeSet<UsdPrice> {
        BTreeSet::new()
    }

    /// Place a resting ask at `price` for `who` with the given size and
    /// reserved margin. ASKS use the un-inverted price as the storage
    /// key — no `may_invert_price` shim needed on this side.
    fn save_limit_ask(
        storage: &mut dyn Storage,
        who: Addr,
        size: i128,
        reserved_margin: i128,
        price: i128,
        order_id: u64,
    ) {
        let key = (pair_id(), UsdPrice::new_int(price), Uint64::new(order_id));
        let order = LimitOrder {
            user: who,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(reserved_margin),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
            client_order_id: None,
        };
        ASKS.save(storage, key, &order).unwrap();
    }

    /// Seed a user state carrying the order-book accounting for one
    /// pending ask: reserved margin and open-order count.
    fn save_user_with_order(storage: &mut dyn Storage, who: Addr, reserved_margin: i128) {
        USER_STATES
            .save(storage, who, &UserState {
                reserved_margin: UsdValue::new_int(reserved_margin),
                open_order_count: 1,
                ..Default::default()
            })
            .unwrap();
    }

    /// A sub-lot ask (size 3, lot_size 5) is cancelled with a full margin
    /// refund — `reserved_margin` returns to zero, `open_order_count`
    /// decrements, and the order disappears from `ASKS`.
    #[test]
    fn update_cancels_sub_lot_ask() {
        let mut storage = MockStorage::new();
        save_pair_state(&mut storage, 0, 0);
        // Ask size is negative (sell), magnitude 3 < lot_size 5.
        save_limit_ask(&mut storage, addr(1), -3, 100, 1_000, 1);
        save_user_with_order(&mut storage, addr(1), 100);

        update_lot_size(
            &mut storage,
            &pair_id(),
            Quantity::ZERO,
            Quantity::new_int(5),
            &empty_buckets(),
            None,
        )
        .unwrap();

        let user = USER_STATES.load(&storage, addr(1)).unwrap();
        assert_eq!(user.reserved_margin, UsdValue::ZERO);
        assert_eq!(user.open_order_count, 0);
        assert!(
            !ASKS.has(
                &storage,
                (pair_id(), UsdPrice::new_int(1_000), Uint64::new(1))
            ),
            "sub-lot ask not removed",
        );
    }

    /// A partially-aligned ask (size 10, lot_size 4) is shrunk in place
    /// to 8 (= 2 lots). Reserved margin scales by 8/10 — the freed slice
    /// (20% of 100 = 20) is released back to the user. The order stays
    /// on the book and `open_order_count` is unchanged.
    #[test]
    fn update_shrinks_partial_truncation_ask() {
        let mut storage = MockStorage::new();
        save_pair_state(&mut storage, 0, 0);
        save_limit_ask(&mut storage, addr(1), -10, 100, 1_000, 1);
        save_user_with_order(&mut storage, addr(1), 100);

        // Pre-condition: the user has 100 of reserved margin locked
        // behind the resting ask. The test then verifies the migration
        // releases 20 of it (the freed 20% slice).
        let user_before = USER_STATES.load(&storage, addr(1)).unwrap();
        assert_eq!(user_before.reserved_margin, UsdValue::new_int(100));
        let order_before = ASKS
            .load(
                &storage,
                (pair_id(), UsdPrice::new_int(1_000), Uint64::new(1)),
            )
            .unwrap();
        assert_eq!(order_before.reserved_margin, UsdValue::new_int(100));

        update_lot_size(
            &mut storage,
            &pair_id(),
            Quantity::ZERO,
            Quantity::new_int(4),
            &empty_buckets(),
            None,
        )
        .unwrap();

        let key = (pair_id(), UsdPrice::new_int(1_000), Uint64::new(1));
        let order = ASKS.load(&storage, key).unwrap();
        assert_eq!(order.size, Quantity::new_int(-8));
        assert_eq!(order.reserved_margin, UsdValue::new_int(80));

        let user = USER_STATES.load(&storage, addr(1)).unwrap();
        assert_eq!(user.reserved_margin, UsdValue::new_int(80));
        // Order stays on the book, so the count is unchanged.
        assert_eq!(user.open_order_count, 1);
    }

    // ---- top-level entry point ----

    /// Skip condition fires when the new lot divides the old one
    /// (10 → 5): every pass is bypassed. We seed an OI imbalance to
    /// prove the rebalance pass — which would trim the unmatched long —
    /// never runs.
    #[test]
    fn update_skips_when_new_divides_old() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), 10);
        save_pair_state(&mut storage, 10, 0);

        update_lot_size(
            &mut storage,
            &pair_id(),
            Quantity::new_int(10),
            Quantity::new_int(5),
            &empty_buckets(),
            None,
        )
        .unwrap();

        // Long position survives the unbalanced OI because the skip
        // path bypasses the rebalance pass entirely.
        assert_eq!(
            load_position_size(&storage, addr(1)),
            Some(Quantity::new_int(10))
        );
        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::new_int(10));
        assert_eq!(pair_state.short_oi, Quantity::ZERO);
    }

    /// Idempotent: running the migration a second time with the same
    /// `new_lot_size` is a no-op because everything is already lot-aligned.
    #[test]
    fn update_idempotent_second_run_noop() {
        let mut storage = MockStorage::new();
        save_position(&mut storage, addr(1), 7);
        save_pair_state(&mut storage, 7, 0);

        // First run: truncates 7 → 5.
        update_lot_size(
            &mut storage,
            &pair_id(),
            Quantity::ZERO,
            Quantity::new_int(5),
            &empty_buckets(),
            None,
        )
        .unwrap();
        // After first run: long_oi must equal short_oi (rebalance).
        // No shorts seeded, so the rebalance pass trims the single long
        // down to zero. That leaves no position and OI 0/0.
        assert_eq!(load_position_size(&storage, addr(1)), None);

        // Second run: new == old → divisibility skip fires (`new % new == 0`).
        update_lot_size(
            &mut storage,
            &pair_id(),
            Quantity::new_int(5),
            Quantity::new_int(5),
            &empty_buckets(),
            None,
        )
        .unwrap();
        // Still no position, OI still balanced.
        assert_eq!(load_position_size(&storage, addr(1)), None);
        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, pair_state.short_oi);
    }

    /// End-to-end: a sub-lot ask, a misaligned long position, and a
    /// misaligned short position. After `update_lot_size`:
    ///   - Ask is cancelled, user-1 reserved margin returns to zero.
    ///   - Long (7) truncates to 5; short (-7) truncates to -5.
    ///   - OI ends balanced at 5/5 — no rebalance pass needed because
    ///     truncation already lined them up symmetrically.
    #[test]
    fn update_handles_orders_positions_and_oi_together() {
        let mut storage = MockStorage::new();
        save_pair_state(&mut storage, 7, 7);
        // Long at addr(1) with a sub-lot ask on the side.
        save_position(&mut storage, addr(1), 7);
        save_limit_ask(&mut storage, addr(1), -3, 50, 2_000, 1);
        // Augment the user state save_position wrote with the order's
        // reserved margin and open-order count.
        let mut user_1 = USER_STATES.load(&storage, addr(1)).unwrap();
        user_1.reserved_margin = UsdValue::new_int(50);
        user_1.open_order_count = 1;
        USER_STATES.save(&mut storage, addr(1), &user_1).unwrap();
        // Short at addr(2).
        save_position(&mut storage, addr(2), -7);

        update_lot_size(
            &mut storage,
            &pair_id(),
            Quantity::ZERO,
            Quantity::new_int(5),
            &empty_buckets(),
            None,
        )
        .unwrap();

        // Order: cancelled, full refund.
        assert!(!ASKS.has(
            &storage,
            (pair_id(), UsdPrice::new_int(2_000), Uint64::new(1))
        ));
        let user_1 = USER_STATES.load(&storage, addr(1)).unwrap();
        assert_eq!(user_1.reserved_margin, UsdValue::ZERO);
        assert_eq!(user_1.open_order_count, 0);

        // Positions: truncated symmetrically.
        assert_eq!(
            load_position_size(&storage, addr(1)),
            Some(Quantity::new_int(5))
        );
        assert_eq!(
            load_position_size(&storage, addr(2)),
            Some(Quantity::new_int(-5))
        );

        // OI: balanced at one lot per side. The rebalance pass had
        // nothing to do because long_oi == short_oi after truncation.
        let pair_state = PAIR_STATES.load(&storage, &pair_id()).unwrap();
        assert_eq!(pair_state.long_oi, Quantity::new_int(5));
        assert_eq!(pair_state.short_oi, Quantity::new_int(5));
    }
}
