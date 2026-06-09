//! Generic price-time-priority matching engine.
//!
//! `walk_book` iterates one side of the order book in price-time priority,
//! producing two streams of records:
//!
//! - `RawFill` — a maker order intersected with the taker's remaining size,
//!   priced at the maker's stored price.
//! - `RemovedMaker` — a maker order the engine walked past without filling
//!   because it tripped a generic guard (self-trade prevention or the
//!   resting price drifted outside the pair's `max_limit_price_deviation`
//!   band).
//!
//! Records are emitted in the order in which the walk encountered the
//! corresponding maker — so callers can replay them and produce the same
//! event sequence the original interleaved code did. The walk performs
//! **no storage writes** and **no perp-specific settlement**: the caller
//! is responsible for releasing reserved margin, updating positions,
//! computing fees / PnL / funding, decrementing liquidity depth, and
//! removing or rewriting maker orders in the `BIDS` / `ASKS` maps.

use {
    crate::{
        ASKS, BIDS, ClientOrderId, Dimensionless, LimitOrder, OrderId, PairId, Quantity,
        ReasonForOrderRemoval, TriggerDirection, UsdPrice, check_price_band, decompose_fill,
        is_price_constraint_violated, may_invert_price,
    },
    grug_types::{Addr, Order as IterationOrder, StdResult, Storage},
    std::collections::BTreeMap,
};

/// One match produced by [`walk_book`] — pure data, no perp settlement
/// applied. The caller turns each entry into the perp-specific side
/// effects: `settle_fill` for both maker and taker, fee / PnL credit,
/// position update, OI bookkeeping, liquidity-depth decrement, removing
/// or rewriting the maker order in `BIDS` / `ASKS`, and emitting the
/// `OrderFilled` event pair.
#[derive(Debug, Clone)]
pub struct RawFill {
    /// The maker order's id, used to look it up in `BIDS` / `ASKS`.
    pub maker_order_id: OrderId,

    /// Owner of the maker order. Used by the caller to look up
    /// `UserState`, charge the maker fee, and update margin / position.
    pub maker_addr: Addr,

    /// Caller-assigned id from the maker's originally-submitted order
    /// (or `None`). Surfaced on the maker side's `OrderFilled` event so
    /// off-chain consumers can correlate the fill with the order.
    pub maker_client_order_id: Option<ClientOrderId>,

    /// `|size|` of the maker order *before* this fill is applied.
    /// Equals the absolute size as stored in `BIDS` / `ASKS` at the
    /// instant the engine inspected it. The caller passes this to
    /// `decrease_liquidity_depths` to remove the maker's prior depth
    /// contribution before re-adding the post-fill remainder (see
    /// `liquidity_depth.rs::partial_fill_no_residual_depth` for why).
    pub maker_pre_fill_size: Quantity,

    /// `|size|` of the maker order *after* this fill is applied. Zero
    /// iff the maker was fully filled — the caller should remove the
    /// order from storage in that case, otherwise rewrite it.
    pub maker_post_fill_size: Quantity,

    /// Stored price of the maker order in `BIDS` / `ASKS`. For bids
    /// this is the inverted price; the caller can pass it back to
    /// `may_invert_price` if it needs the original.
    pub maker_stored_price: UsdPrice,

    /// "Real" (un-inverted) price at which this fill executes — the
    /// resting price of the maker order.
    pub fill_price: UsdPrice,

    /// Signed taker-perspective fill size: positive when the taker is
    /// the bid (buying), negative when the taker is the ask (selling).
    /// The maker's signed fill size is `-fill_size`.
    pub fill_size: Quantity,

    /// Snapshot of the maker order's state immediately *before* this
    /// fill is applied. Carries the maker's `reserved_margin`,
    /// `tp` / `sl` child orders, and the post-fill remainder when it
    /// is non-zero — the caller releases the proportional margin,
    /// applies child orders to the resulting position, and rewrites
    /// the order if it isn't fully filled.
    pub maker_order: LimitOrder,
}

/// A maker order the engine walked past without filling. The caller
/// decides what perp-specific cleanup to do (release reserved margin,
/// decrement the maker's `open_order_count`), then removes the order
/// from `BIDS` / `ASKS`, decrements liquidity depth by the order's
/// pre-fill absolute size, and emits `OrderRemoved` with `reason`.
#[derive(Debug, Clone)]
pub struct RemovedMaker {
    /// The maker order's id, for storage removal and the
    /// `OrderRemoved` event.
    pub maker_order_id: OrderId,

    /// Owner of the maker order. The caller updates this user's
    /// `UserState` (release reserved margin, decrement
    /// `open_order_count`).
    pub maker_addr: Addr,

    /// Caller-assigned id, surfaced on the `OrderRemoved` event.
    pub maker_client_order_id: Option<ClientOrderId>,

    /// `|size|` of the maker order at the time of removal. The order
    /// was not filled, so this is also the size whose depth
    /// contribution must be removed.
    pub maker_pre_fill_size: Quantity,

    /// Stored price of the maker order in `BIDS` / `ASKS`.
    pub maker_stored_price: UsdPrice,

    /// "Real" (un-inverted) price of the maker order — convenient for
    /// the caller to pass to `decrease_liquidity_depths` without
    /// re-inverting `maker_stored_price`.
    pub maker_real_price: UsdPrice,

    /// Snapshot of the maker order's state at the time of removal.
    /// Carries the `reserved_margin` the caller releases.
    pub maker_order: LimitOrder,

    /// Why the engine walked past this maker. Either
    /// [`ReasonForOrderRemoval::SelfTradePrevention`] (the maker is
    /// the taker himself) or
    /// [`ReasonForOrderRemoval::PriceBandViolation`] (the resting
    /// price drifted out of band as the oracle moved since the
    /// maker was placed). The caller writes this verbatim into the
    /// `OrderRemoved` event so STP and out-of-band cancels stay
    /// distinguishable in the event stream.
    pub reason: ReasonForOrderRemoval,
}

/// One step of the walk: either a fill against a maker, or a
/// "walked-past" removal. Stored in chronological encounter order
/// so the caller can emit `OrderFilled` and `OrderRemoved` events in
/// the same sequence the legacy interleaved engine did.
#[derive(Debug, Clone)]
pub enum WalkStep {
    Fill(RawFill),
    Removed(RemovedMaker),
}

/// Owned outcome of a [`walk_book`] call. Carries the chronological
/// step list plus the unfilled remainder.
#[derive(Debug)]
pub struct WalkBookOutcome {
    /// Chronological list of fills and walked-past removals. Caller
    /// iterates in order to produce the same event sequence as the
    /// legacy interleaved matching engine.
    pub steps: Vec<WalkStep>,

    /// Signed taker-perspective remaining size after the walk. Zero
    /// iff the order was fully filled. For partial fills the caller
    /// either parks the remainder as a resting limit order
    /// (`TimeInForce::GoodTilCanceled`) or discards it (`Market` /
    /// `ImmediateOrCancel`).
    pub remaining_size: Quantity,
}

/// Walk the maker side of the book in price-time priority, producing
/// fills and "walked-past" removals as data. **Pure with respect to
/// storage** — performs no writes; the caller applies all side effects
/// from `outcome.steps`.
///
/// # Arguments
///
/// - `taker` — the taker's address; resting orders owned by this
///   address trigger self-trade prevention (EXPIRE_MAKER mode).
/// - `vault_addr` — the perps vault contract address; vault-owned
///   resting orders are exempt from the `max_limit_price_deviation`
///   re-check because their prices are algorithmically bounded by the
///   vault's spread parameters and refreshed on every oracle update.
/// - `taker_is_bid` — `true` if the taker is buying; selects `ASKS`
///   as the maker side.
/// - `target_price` — the worst price the taker is willing to accept;
///   for limit orders this is the limit price, for market orders the
///   slippage-bounded price computed from the oracle.
/// - `oracle_price` — current oracle price, used by the price-band
///   re-check.
/// - `max_limit_price_deviation` — pair parameter; out-of-band
///   resting limits are cancelled and the walk continues deeper.
/// - `remaining_size` — signed taker-perspective size to fill. The
///   walk decrements this in place and returns the unfilled remainder
///   in `outcome.remaining_size`.
/// - `maker_position` — returns a maker's *current* signed position size
///   in this pair (positive long, negative short, zero if none). Used to
///   re-clamp reduce-only maker orders so they can only move the maker's
///   position toward zero, never grow or flip it. Must report positions
///   from the same source the caller settles against, so the walk's
///   simulated positions match what settlement actually applies.
///
/// # Reduce-only makers
///
/// A reduce-only maker order may only reduce the maker's position. Its size
/// was clamped to the position when it was placed, but the position may have
/// shrunk since (other fills, liquidation, ADL), and one taker order can
/// sweep several of the same maker's orders. So each reduce-only maker fill
/// is re-clamped here against the maker's *current* position (the pre-walk
/// position from `maker_position` plus the fills already applied earlier in
/// this same walk). A reduce-only order with nothing left to close is
/// skipped and left resting — it must never fill, as that would open the
/// opposite side.
///
/// # Termination
///
/// The walk stops when any of these holds:
///
/// 1. `remaining_size` reaches zero — the taker is fully filled;
/// 2. the next maker's resting price is worse than `target_price` for
///    the taker (uses [`is_price_constraint_violated`]);
/// 3. the maker side of the book is exhausted.
#[allow(clippy::too_many_arguments)]
pub fn walk_book(
    storage: &dyn Storage,
    pair_id: &PairId,
    taker: Addr,
    vault_addr: Addr,
    taker_is_bid: bool,
    target_price: UsdPrice,
    oracle_price: UsdPrice,
    max_limit_price_deviation: Dimensionless,
    mut remaining_size: Quantity,
    maker_position: impl Fn(Addr) -> StdResult<Quantity>,
) -> StdResult<WalkBookOutcome> {
    let maker_book = if taker_is_bid {
        ASKS
    } else {
        BIDS
    };

    let maker_orders =
        maker_book
            .prefix(pair_id.clone())
            .range(storage, None, None, IterationOrder::Ascending);

    let mut steps: Vec<WalkStep> = Vec::new();

    // Cumulative maker-side fill applied so far in this walk, per maker (the
    // maker fills the opposite of the taker: -taker_fill_size). Used by the
    // reduce-only clamp below: a maker's current position is the pre-walk
    // position from `maker_position` plus this delta. Tracked for *every*
    // filled maker so a later reduce-only order of the same maker reflects
    // earlier fills, including non-reduce-only ones.
    let mut maker_fill_deltas: BTreeMap<Addr, Quantity> = BTreeMap::new();

    for record in maker_orders {
        let ((stored_price, maker_order_id), maker_order) = record?;

        // If the maker is bid (i.e. taker is ask), un-invert the price.
        let resting_price = may_invert_price(stored_price, !taker_is_bid);

        // ----------------------- Termination conditions ----------------------

        if remaining_size.is_zero() {
            break;
        }

        if is_price_constraint_violated(resting_price, target_price, taker_is_bid) {
            break;
        }

        // ----------------------- Self-trade prevention -----------------------
        //
        // EXPIRE_MAKER mode: cancel the maker and continue.
        // <https://developers.binance.com/docs/binance-spot-api-docs/faqs/stp_faq>
        if maker_order.user == taker {
            let pre_fill_abs_size = maker_order.size.checked_abs()?;

            steps.push(WalkStep::Removed(RemovedMaker {
                maker_order_id,
                maker_addr: maker_order.user,
                maker_client_order_id: maker_order.client_order_id,
                maker_pre_fill_size: pre_fill_abs_size,
                maker_stored_price: stored_price,
                maker_real_price: resting_price,
                maker_order,
                reason: ReasonForOrderRemoval::SelfTradePrevention,
            }));

            continue;
        }

        // ----------------------- Price-band re-check -------------------------
        //
        // The maker's price was within band when placed, but the oracle may
        // have drifted since. Cancel out-of-band makers and walk deeper.
        //
        // Vault quotes are exempt — their prices are algorithmically bounded
        // by `vault_half_spread * (1 + vault_spread_skew_factor)` and
        // refreshed on every oracle update, so cancelling them during
        // matching would cause continuous churn without any security gain
        // (the vault cannot be part of an attacker's coordinated setup).
        if maker_order.user != vault_addr
            && check_price_band(resting_price, oracle_price, max_limit_price_deviation).is_err()
        {
            let pre_fill_abs_size = maker_order.size.checked_abs()?;

            steps.push(WalkStep::Removed(RemovedMaker {
                maker_order_id,
                maker_addr: maker_order.user,
                maker_client_order_id: maker_order.client_order_id,
                maker_pre_fill_size: pre_fill_abs_size,
                maker_stored_price: stored_price,
                maker_real_price: resting_price,
                maker_order,
                reason: ReasonForOrderRemoval::PriceBandViolation,
            }));

            continue;
        }

        // ---------------------- Determine fillable size ----------------------
        //
        // The maker's signed size is the negative of what the taker needs:
        // a maker bid has positive size and matches a taker ask (negative
        // remaining_size), and vice versa.
        //
        // Reduce-only makers are additionally clamped to their current
        // position (see the "Reduce-only makers" doc section): only the
        // portion of the order that closes the maker's position is fillable,
        // computed against the pre-walk position plus the fills already
        // applied to this maker earlier in the walk.

        let maker_user = maker_order.user;

        let fillable_maker_size = if maker_order.reduce_only {
            let delta = maker_fill_deltas
                .get(&maker_user)
                .copied()
                .unwrap_or(Quantity::ZERO);
            let current_position = maker_position(maker_user)?.checked_add(delta)?;

            let (closing, _opening) = decompose_fill(maker_order.size, current_position);
            closing
        } else {
            maker_order.size
        };

        // A reduce-only order with nothing left to close is inert: skip it,
        // leaving it resting (cleanup of such dead/oversized orders is done
        // separately via dynamic re-sizing on position changes). It must not
        // fill — that would open a new position.
        if fillable_maker_size.is_zero() {
            continue;
        }

        let opposite = fillable_maker_size.checked_neg()?;

        let taker_fill_size = if taker_is_bid {
            remaining_size.min(opposite)
        } else {
            remaining_size.max(opposite)
        };

        let pre_fill_abs_size = maker_order.size.checked_abs()?;

        // post-fill maker size is `maker_order.size - maker_fill_size`,
        // and `maker_fill_size = -taker_fill_size`, so:
        let post_fill_signed = maker_order
            .size
            .checked_sub(taker_fill_size.checked_neg()?)?;
        let post_fill_abs = post_fill_signed.checked_abs()?;

        steps.push(WalkStep::Fill(RawFill {
            maker_order_id,
            maker_addr: maker_user,
            maker_client_order_id: maker_order.client_order_id,
            maker_pre_fill_size: pre_fill_abs_size,
            maker_post_fill_size: post_fill_abs,
            maker_stored_price: stored_price,
            fill_price: resting_price,
            fill_size: taker_fill_size,
            maker_order,
        }));

        // Record this fill against the maker's running position so a later
        // reduce-only order of the same maker clamps against the reduced
        // size. The maker fills the opposite of the taker: -taker_fill_size.
        maker_fill_deltas
            .entry(maker_user)
            .or_insert(Quantity::ZERO)
            .checked_add_assign(taker_fill_size.checked_neg()?)?;

        remaining_size.checked_sub_assign(taker_fill_size)?;
    }

    Ok(WalkBookOutcome {
        steps,
        remaining_size,
    })
}

/// Is the conditional order's trigger condition met by the current oracle
/// price?
///
/// - [`crate::TriggerDirection::Above`]: triggers when
///   `oracle_price >= trigger_price` (TP for longs, SL for shorts).
/// - [`crate::TriggerDirection::Below`]: triggers when
///   `oracle_price <= trigger_price` (SL for longs, TP for shorts).
pub fn is_conditional_order_triggered(
    trigger_price: UsdPrice,
    trigger_direction: crate::TriggerDirection,
    oracle_price: UsdPrice,
) -> bool {
    match trigger_direction {
        TriggerDirection::Above => oracle_price >= trigger_price,
        TriggerDirection::Below => oracle_price <= trigger_price,
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, test_case::test_case};

    // (trigger_price, direction, oracle_price, expected)
    #[test_case(100, TriggerDirection::Above,  99, false ; "above oracle below trigger")]
    #[test_case(100, TriggerDirection::Above, 100, true  ; "above oracle equals trigger")]
    #[test_case(100, TriggerDirection::Above, 101, true  ; "above oracle above trigger")]
    #[test_case(100, TriggerDirection::Below,  99, true  ; "below oracle below trigger")]
    #[test_case(100, TriggerDirection::Below, 100, true  ; "below oracle equals trigger")]
    #[test_case(100, TriggerDirection::Below, 101, false ; "below oracle above trigger")]
    fn is_conditional_order_triggered_works(
        trigger: i128,
        direction: TriggerDirection,
        oracle: i128,
        expected: bool,
    ) {
        assert_eq!(
            is_conditional_order_triggered(
                UsdPrice::new_int(trigger),
                direction,
                UsdPrice::new_int(oracle)
            ),
            expected
        );
    }
}
