//! One-shot, chain-scoped state repair for a self-trade-prevention (STP)
//! leak that corrupted a single user's `UserState` on testnet.
//!
//! # Background
//!
//! Prior to the fix in `cron::process_triggered_order`, when a user's
//! triggered conditional order submitted a market close that hit STP
//! against their own resting order and then bailed out with "no
//! liquidity at acceptable price!", the error path swallowed the error
//! and persisted a `UserState` whose `open_order_count` and
//! `reserved_margin` had both been decremented in-memory by the STP
//! branch — without the corresponding `BIDS`/`ASKS` removal ever
//! running. Each incident leaked the full reserved margin of one
//! cancelled-but-still-on-book order out of the user's `reserved_margin`
//! and subtracted one from their `open_order_count`.
//!
//! Over multiple incidents, user `0xfce3ef4f6ec8b31bd9e4feee43e645128948895a`
//! drifted to `open_order_count = 0` and `reserved_margin ≈ -$177,831`
//! (physically impossible) while their actual resting orders on the book
//! sum to `2` orders / `+$34,037.591919`. The next `match_order` call
//! that fills one of those orders panics with "attempt to subtract with
//! overflow" at `maker_state.open_order_count -= 1`, because
//! `maker_state.open_order_count` is loaded from storage at `0`.
//!
//! See:
//! - the regression test
//!   `conditional_order_self_trade_failure_preserves_user_state` in
//!   `dango/testing/tests/perps/conditional_orders.rs`, and
//! - the offline diagnostic script
//!   `dango/scripts/examples/analyze_perps_inconsistency.rs`.
//!
//! # Repair
//!
//! On the first call to `apply_fix` *on the testnet chain* *at or after*
//! `TESTNET_FIX_MIN_HEIGHT`, recompute the corrupted user's
//! `open_order_count` and `reserved_margin` from the authoritative
//! `BIDS` + `ASKS` tallies and overwrite the two fields on their
//! `UserState`. All other fields (`margin`, `positions`, `unlocks`,
//! `vault_shares`) are left untouched — the STP-leak path in
//! `match_order` only mutates those two fields in the ensure-fail path,
//! so no other state needs repair. Secondary `UserStateIndexes`
//! (`earliest_unlock_end_time`, `conditional_orders`) don't index
//! `open_order_count` or `reserved_margin`, so `USER_STATES.save` won't
//! move any index entries.
//!
//! Idempotency is guarded three ways, any one of which is sufficient:
//! - `chain_id != TESTNET_CHAIN_ID` → no-op (mainnet is never touched).
//! - `height < TESTNET_FIX_MIN_HEIGHT` → no-op.
//! - `FIX_ALREADY_APPLIED` storage flag set → no-op.
//!
//! `apply_fix` is invoked from both `execute` and `cron_execute`. In a
//! normal `do_finalize_block` ordering, all user txs (`execute`) run
//! before cron jobs (`cron_execute`) within a block, so a single
//! `execute`-level call would usually be enough — but if the upgrade
//! block happens to contain zero perps-targeting txs, the cron would
//! hit the stale state before the repair ran. Calling from both entry
//! points closes that gap; the storage flag makes the second call a
//! trivial no-op.
//!
//! # Scope
//!
//! This repair intentionally does NOT touch the ~108 users who show
//! sub-cent `reserved_margin` drift on the book. That drift has a
//! different root cause (partial-fill margin-release truncation at
//! `match_order`'s `margin × fill_size ÷ order_size`) and is tracked
//! separately — folding it into this repair would muddle the audit
//! trail for this specific incident.

use {
    crate::state::{ASKS, BIDS, USER_STATES},
    dango_types::UsdValue,
    grug::{Addr, Item, Order as IterationOrder, StdResult, Storage, addr},
};

/// The repair only runs on the testnet chain. Any other chain (including
/// mainnet) is a no-op.
const TESTNET_CHAIN_ID: &str = "dango-testnet-1";

/// Minimum block height at which the repair is allowed to run. The repair
/// will execute on the first `execute` or `cron_execute` call at or after
/// this height; it does not need to hit the exact height. The flag below
/// then prevents any subsequent re-runs.
const TESTNET_FIX_MIN_HEIGHT: u64 = 21991499;

/// The one user whose `UserState` was corrupted by the STP leak. Identified
/// by offline diagnosis of the node's RocksDB snapshot (see
/// `dango/scripts/examples/analyze_perps_inconsistency.rs`).
const STALE_USER: Addr = addr!("fce3ef4f6ec8b31bd9e4feee43e645128948895a");

/// Idempotency guard. Written once, on the first successful run of the
/// repair, and checked on every subsequent invocation. Scoped to this
/// contract's sub-storage.
const FIX_ALREADY_APPLIED: Item<bool> = Item::new("fix_stp_leak_applied");

/// Entry point called at the top of `execute` and `cron_execute`. On
/// testnet at the upgrade height, recomputes and rewrites the corrupted
/// user's `open_order_count` / `reserved_margin`. Elsewhere, a fast no-op.
pub fn apply_fix(storage: &mut dyn Storage, chain_id: &str, height: u64) -> StdResult<()> {
    // Mainnet (and any other chain) → no-op.
    if chain_id != TESTNET_CHAIN_ID {
        tracing::info!(
            chain_id,
            TESTNET_CHAIN_ID,
            "Skipping fix: chain ID != TESTNET_CHAIN_ID"
        );

        return Ok(());
    }

    // Pre-upgrade blocks on testnet → no-op. After the upgrade height, the
    // first invocation falls through and the storage flag prevents re-runs.
    if height < TESTNET_FIX_MIN_HEIGHT {
        tracing::info!(
            height,
            TESTNET_FIX_MIN_HEIGHT,
            "Skipping fix: height < TESTNET_FIX_MIN_HEIGHT"
        );

        return Ok(());
    }

    // Already applied → no-op.
    if FIX_ALREADY_APPLIED.may_load(storage)?.is_some() {
        tracing::info!("Skipping fix: fix already applied");

        return Ok(());
    }

    // --- Recompute authoritative values from the order book. ---
    //
    // The `user` secondary index on `BIDS`/`ASKS` lets us enumerate only
    // this user's orders without a full book scan.
    let mut actual_open_order_count: usize = 0;
    let mut actual_reserved_margin = UsdValue::ZERO;

    for res in
        BIDS.idx
            .user
            .prefix(STALE_USER)
            .range(storage, None, None, IterationOrder::Ascending)
    {
        let (_order_key, order) = res?;
        actual_open_order_count += 1;
        actual_reserved_margin.checked_add_assign(order.reserved_margin)?;
    }
    for res in
        ASKS.idx
            .user
            .prefix(STALE_USER)
            .range(storage, None, None, IterationOrder::Ascending)
    {
        let (_order_key, order) = res?;
        actual_open_order_count += 1;
        actual_reserved_margin.checked_add_assign(order.reserved_margin)?;
    }

    // --- Load, overwrite the two fields, save. ---
    //
    // `USER_STATES` is an `IndexedMap`; using `save` re-indexes safely.
    // Neither of its secondary indexes (`earliest_unlock_end_time`,
    // `conditional_orders`) depend on `open_order_count` or
    // `reserved_margin`, so no index entries will actually move.
    let mut user_state = USER_STATES
        .may_load(storage, STALE_USER)?
        .unwrap_or_default();

    #[cfg(feature = "tracing")]
    let prev_open_order_count = user_state.open_order_count;
    #[cfg(feature = "tracing")]
    let prev_reserved_margin = user_state.reserved_margin;

    user_state.open_order_count = actual_open_order_count;
    user_state.reserved_margin = actual_reserved_margin;

    USER_STATES.save(storage, STALE_USER, &user_state)?;

    // Set the flag LAST so that any error in the repair above leaves the
    // flag unset and we retry on the next call.
    FIX_ALREADY_APPLIED.save(storage, &true)?;

    #[cfg(feature = "tracing")]
    tracing::warn!(
        user = %STALE_USER,
        prev_open_order_count,
        new_open_order_count = actual_open_order_count,
        %prev_reserved_margin,
        new_reserved_margin = %actual_reserved_margin,
        "!!! Applied one-shot state repair for STP-leaked UserState !!!"
    );

    Ok(())
}
