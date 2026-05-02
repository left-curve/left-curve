//! Generic order-removal primitive shared between cancel and matching paths.

use {
    crate::{
        LimitOrder, OrderRemoved, PairId, ReasonForOrderRemoval, decrease_liquidity_depths,
        may_invert_price,
        state::{ASKS, BIDS, OrderKey},
    },
    grug::{EventBuilder, StdResult, Storage},
    std::collections::BTreeSet,
};

/// Remove a resting order from `BIDS` / `ASKS`, decrement its liquidity
/// depth contribution, and (optionally) emit an [`OrderRemoved`] event.
///
/// Generic w.r.t. perp settlement: the caller is responsible for the
/// user-state side of the cancellation (releasing reserved margin,
/// decrementing `open_order_count`) before invoking this function.
///
/// # Behavior
///
/// 1. Decrements depth in every configured `bucket_size`, using the
///    order's pre-removal absolute size.
/// 2. Removes the order from `BIDS` if `is_bid`, otherwise from `ASKS`.
/// 3. If `events` is `Some`, pushes `OrderRemoved { reason, .. }`.
pub fn remove_order(
    storage: &mut dyn Storage,
    order_key: OrderKey,
    order: &LimitOrder,
    reason: ReasonForOrderRemoval,
    bucket_sizes: &BTreeSet<crate::UsdPrice>,
    events: Option<&mut EventBuilder>,
) -> StdResult<()> {
    let (pair_id, stored_price, order_id): (PairId, _, _) = order_key.clone();
    let is_bid = order.size.is_positive();
    let real_price = may_invert_price(stored_price, is_bid);

    decrease_liquidity_depths(
        storage,
        &pair_id,
        is_bid,
        real_price,
        order.size.checked_abs()?,
        bucket_sizes,
    )?;

    if is_bid {
        BIDS.remove(storage, order_key)?;
    } else {
        ASKS.remove(storage, order_key)?;
    }

    if let Some(events) = events {
        events.push(OrderRemoved {
            order_id,
            pair_id,
            user: order.user,
            reason,
            client_order_id: order.client_order_id,
        })?;
    }

    Ok(())
}
