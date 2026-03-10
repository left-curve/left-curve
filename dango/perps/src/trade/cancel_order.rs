use {
    crate::{
        ASKS, BIDS, OrderKey, PAIR_PARAMS, USER_STATES, liquidity_depth::decrease_liquidity_depths,
        price::may_invert_price,
    },
    anyhow::{anyhow, ensure},
    dango_types::perps::{
        Order, OrderId, OrderRemoved, PairId, PairParam, ReasonForOrderRemoval, UserState,
    },
    grug::{Addr, EventBuilder, MutableCtx, Order as IterationOrder, Response, StdResult, Storage},
    std::collections::{BTreeMap, BTreeSet},
};

pub fn cancel_one_order(ctx: MutableCtx, order_id: OrderId) -> anyhow::Result<Response> {
    // Since we don't know whether it's a buy or a sell order, we first attempt
    // to load it from the `BIDS` map. If not found, load it from `ASKS`.
    // If still not found, bail.
    let (order_key, order) = BIDS
        .idx
        .order_id
        .may_load(ctx.storage, order_id)
        .transpose()
        .or_else(|| {
            ASKS.idx
                .order_id
                .may_load(ctx.storage, order_id)
                .transpose()
        })
        .ok_or_else(|| anyhow!("order not found with id {order_id}"))??;

    ensure!(
        ctx.sender == order.user,
        "you are not the owner of this order"
    );

    let mut events = EventBuilder::new();

    update_user_state_with(ctx.storage, ctx.sender, |storage, user_state| {
        _cancel_one_order(
            storage,
            user_state,
            order_key,
            order,
            Some(&mut events),
            ReasonForOrderRemoval::Canceled,
            |storage, pair_id| PAIR_PARAMS.load(storage, pair_id),
        )
    })?;

    Ok(Response::new().add_events(events)?)
}

/// Mutates:
///
/// - User state: releases reserved margin, decrement open order count.
///   Does NOT save updated user state to storage. Closing this one order may be
///   only step in a bigger routine -- the caller may want to do other changes
///   to the user state before saving.
/// - Remove the order from the `BIDS` or `ASKS` map.
/// - Remove liquidity depth contributed by this order.
fn _cancel_one_order<F>(
    storage: &mut dyn Storage,
    user_state: &mut UserState,
    order_key: OrderKey,
    order: Order,
    events: Option<&mut EventBuilder>,
    reason: ReasonForOrderRemoval,
    pair_param: F,
) -> StdResult<()>
where
    F: FnOnce(&dyn Storage, &PairId) -> StdResult<PairParam>,
{
    let (pair_id, stored_price, order_id) = order_key.clone();
    let is_bid = order.size.is_positive();

    let pair_param = pair_param(storage, &pair_id)?;
    let real_price = may_invert_price(stored_price, is_bid);

    // Update user state.
    (user_state.reserved_margin).checked_sub_assign(order.reserved_margin)?;
    user_state.open_order_count -= 1;

    // Remove liquidity contributed by this order.
    decrease_liquidity_depths(
        storage,
        &pair_id,
        is_bid,
        real_price,
        order.size.checked_abs()?,
        &pair_param.bucket_sizes,
    )?;

    // Remove the order from storage.
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
        })?;
    }

    Ok(())
}

pub fn cancel_all_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    let mut events = EventBuilder::new();

    update_user_state_with(ctx.storage, ctx.sender, |storage, user_state| {
        _cancel_all_orders(
            storage,
            ctx.sender,
            user_state,
            Some(&mut events),
            ReasonForOrderRemoval::Canceled,
        )
    })?;

    Ok(Response::new().add_events(events)?)
}

/// Cancel all resting orders for a user, updating the in-memory `user_state`.
///
/// Writes to `BIDS` / `ASKS` in storage but does **not** persist `user_state`
/// — the caller is responsible for saving or removing it.
pub fn _cancel_all_orders(
    storage: &mut dyn Storage,
    user: Addr,
    user_state: &mut UserState,
    mut events: Option<&mut EventBuilder>,
    reason: ReasonForOrderRemoval,
) -> StdResult<()> {
    // Collect all orders from the caller.
    let bids = BIDS
        .idx
        .user
        .prefix(user)
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    let asks = ASKS
        .idx
        .user
        .prefix(user)
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    // Collect the parameters of all pairs involved, so that we don't need to
    // load it each time we cancel an order (DB read is slow).
    let pair_ids = bids
        .iter()
        .chain(&asks)
        .map(|(order_key, _)| {
            let (pair_id, ..) = order_key;
            pair_id.clone()
        })
        .collect::<BTreeSet<_>>();
    let pair_params = pair_ids
        .into_iter()
        .map(|pair_id| {
            let pp = PAIR_PARAMS.load(storage, &pair_id)?;
            Ok((pair_id, pp))
        })
        .collect::<StdResult<BTreeMap<_, _>>>()?;

    // Now mutate storage: update depths, remove orders, update user state.
    for (order_key, order) in bids.into_iter().chain(asks) {
        _cancel_one_order(
            storage,
            user_state,
            order_key,
            order,
            events.as_deref_mut(),
            reason,
            |_, pair_id| Ok(pair_params[pair_id].clone()),
        )?;
    }

    Ok(())
}

/// 1. Load the user's state.
/// 2. Perform a mutable action on the user state. The action may have side
///    effect on the storage.
/// 3. If the user state becomes empty, delete it from storage; otherwise, save
///    the updated user state to storage.
fn update_user_state_with<F>(storage: &mut dyn Storage, user: Addr, action: F) -> StdResult<()>
where
    F: FnOnce(&mut dyn Storage, &mut UserState) -> StdResult<()>,
{
    let mut user_state = USER_STATES.load(storage, user)?;

    action(storage, &mut user_state)?;

    if user_state.is_empty() {
        USER_STATES.remove(storage, user)
    } else {
        USER_STATES.save(storage, user, &user_state)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            PAIR_PARAMS, USER_STATES,
            state::{ASKS, BIDS, OrderKey},
        },
        dango_types::{
            FundingPerUnit, Quantity, UsdPrice, UsdValue,
            perps::{Order, PairId, PairParam, Position, UserState},
        },
        grug::{Addr, Coins, MockContext, ResultExt, Storage, Uint64, Uint128},
        std::collections::{BTreeMap, VecDeque},
    };

    const USER: Addr = Addr::mock(1);
    const OTHER_USER: Addr = Addr::mock(2);

    fn pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    /// Ensure the pair param exists so depth bookkeeping doesn't fail.
    fn ensure_pair_param(storage: &mut dyn Storage) {
        if PAIR_PARAMS.may_load(storage, &pair_id()).unwrap().is_none() {
            PAIR_PARAMS
                .save(storage, &pair_id(), &PairParam::default())
                .unwrap();
        }
    }

    /// Build an `OrderKey` with fixed defaults for pair, price, and timestamp.
    fn order_key(order_id: u64) -> OrderKey {
        (pair_id(), UsdPrice::new_int(50_000), Uint64::new(order_id))
    }

    /// Save a bid (positive size) into `BIDS`.
    fn save_bid(
        storage: &mut dyn Storage,
        order_id: u64,
        user: Addr,
        size: i128,
        reserved_margin: i128,
    ) {
        ensure_pair_param(storage);
        let key = order_key(order_id);
        let order = Order {
            user,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(reserved_margin),
        };

        BIDS.save(storage, key, &order).unwrap();
    }

    /// Save an ask (negative size) into `ASKS`.
    fn save_ask(
        storage: &mut dyn Storage,
        order_id: u64,
        user: Addr,
        size: i128,
        reserved_margin: i128,
    ) {
        ensure_pair_param(storage);
        let key = order_key(order_id);
        let order = Order {
            user,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(reserved_margin),
        };

        ASKS.save(storage, key, &order).unwrap();
    }

    /// Save a `UserState` with the given order count and reserved margin.
    fn save_user_state(
        storage: &mut dyn Storage,
        user: Addr,
        open_order_count: usize,
        reserved_margin: i128,
        positions: BTreeMap<dango_types::perps::PairId, Position>,
    ) {
        let state = UserState {
            unlocks: VecDeque::new(),
            positions,
            reserved_margin: UsdValue::new_int(reserved_margin),
            open_order_count,
            margin: UsdValue::ZERO,
            vault_shares: Uint128::new(0),
        };

        USER_STATES.save(storage, user, &state).unwrap();
    }

    /// Create a dummy position for testing "user state not empty" scenarios.
    fn dummy_position() -> Position {
        Position {
            size: Quantity::new_int(1),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        }
    }

    // ======================== cancel_one_order tests =========================

    #[test]
    fn cancel_bid_order() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid(&mut ctx.storage, 1, USER, 10, 100);
        save_user_state(&mut ctx.storage, USER, 1, 100, BTreeMap::new());

        cancel_one_order(ctx.as_mutable(), Uint64::new(1)).should_succeed();

        // Bid should be removed.
        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(1))
                .unwrap()
                .is_none()
        );

        // User state should be deleted (is_empty).
        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    #[test]
    fn cancel_ask_order() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_ask(&mut ctx.storage, 1, USER, -10, 100);
        save_user_state(&mut ctx.storage, USER, 1, 100, BTreeMap::new());

        cancel_one_order(ctx.as_mutable(), Uint64::new(1)).should_succeed();

        // Ask should be removed.
        assert!(
            ASKS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(1))
                .unwrap()
                .is_none()
        );

        // User state should be deleted (is_empty).
        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    #[test]
    fn cancel_nonexistent_order() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        cancel_one_order(ctx.as_mutable(), Uint64::new(99))
            .should_fail_with_error("order not found");
    }

    #[test]
    fn cancel_order_not_owner() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid(&mut ctx.storage, 1, OTHER_USER, 10, 100);
        save_user_state(&mut ctx.storage, OTHER_USER, 1, 100, BTreeMap::new());

        cancel_one_order(ctx.as_mutable(), Uint64::new(1)).should_fail_with_error("not the owner");
    }

    #[test]
    fn cancel_one_of_many() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid(&mut ctx.storage, 1, USER, 10, 100);
        save_bid(&mut ctx.storage, 2, USER, 5, 100);
        save_user_state(&mut ctx.storage, USER, 2, 200, BTreeMap::new());

        cancel_one_order(ctx.as_mutable(), Uint64::new(1)).should_succeed();

        // Order 1 should be gone.
        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(1))
                .unwrap()
                .is_none()
        );

        // Order 2 should still exist.
        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(2))
                .unwrap()
                .is_some()
        );

        // User state should have count=1, margin=100.
        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(state.open_order_count, 1);
        assert_eq!(state.reserved_margin, UsdValue::new_int(100));
    }

    #[test]
    fn cancel_bid_with_remaining_position() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid(&mut ctx.storage, 1, USER, 10, 100);

        let mut positions = BTreeMap::new();
        positions.insert("perp/btcusd".parse().unwrap(), dummy_position());
        save_user_state(&mut ctx.storage, USER, 1, 100, positions);

        cancel_one_order(ctx.as_mutable(), Uint64::new(1)).should_succeed();

        // Bid removed.
        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(1))
                .unwrap()
                .is_none()
        );

        // User state still exists because of position.
        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(state.open_order_count, 0);
        assert_eq!(state.reserved_margin, UsdValue::ZERO);
        assert!(!state.is_empty());
    }

    // ======================== cancel_all_orders tests ========================

    #[test]
    fn cancel_all_bids() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid(&mut ctx.storage, 1, USER, 10, 100);
        save_bid(&mut ctx.storage, 2, USER, 5, 50);
        save_bid(&mut ctx.storage, 3, USER, 3, 30);
        save_user_state(&mut ctx.storage, USER, 3, 180, BTreeMap::new());

        cancel_all_orders(ctx.as_mutable()).should_succeed();

        // All bids should be removed.
        for id in 1..=3 {
            assert!(
                BIDS.idx
                    .order_id
                    .may_load(&ctx.storage, Uint64::new(id))
                    .unwrap()
                    .is_none()
            );
        }

        // User state should be deleted.
        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    #[test]
    fn cancel_all_asks() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_ask(&mut ctx.storage, 1, USER, -10, 100);
        save_ask(&mut ctx.storage, 2, USER, -5, 50);
        save_user_state(&mut ctx.storage, USER, 2, 150, BTreeMap::new());

        cancel_all_orders(ctx.as_mutable()).should_succeed();

        // All asks should be removed.
        for id in 1..=2 {
            assert!(
                ASKS.idx
                    .order_id
                    .may_load(&ctx.storage, Uint64::new(id))
                    .unwrap()
                    .is_none()
            );
        }

        // User state should be deleted.
        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    #[test]
    fn cancel_mixed_bids_and_asks() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid(&mut ctx.storage, 1, USER, 10, 100);
        save_bid(&mut ctx.storage, 2, USER, 5, 50);
        save_ask(&mut ctx.storage, 3, USER, -8, 80);
        save_user_state(&mut ctx.storage, USER, 3, 230, BTreeMap::new());

        cancel_all_orders(ctx.as_mutable()).should_succeed();

        // All orders should be removed.
        for id in 1..=2 {
            assert!(
                BIDS.idx
                    .order_id
                    .may_load(&ctx.storage, Uint64::new(id))
                    .unwrap()
                    .is_none()
            );
        }
        assert!(
            ASKS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(3))
                .unwrap()
                .is_none()
        );

        // User state should be deleted.
        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    #[test]
    fn cancel_all_no_user_state() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        // No orders, no user state — should error on load.
        cancel_all_orders(ctx.as_mutable()).should_fail();
    }

    #[test]
    fn cancel_all_preserves_other_users() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid(&mut ctx.storage, 1, USER, 10, 100);
        save_bid(&mut ctx.storage, 2, USER, 5, 50);
        save_user_state(&mut ctx.storage, USER, 2, 150, BTreeMap::new());

        // OTHER_USER has their own bid.
        save_bid(&mut ctx.storage, 3, OTHER_USER, 7, 70);
        save_user_state(&mut ctx.storage, OTHER_USER, 1, 70, BTreeMap::new());

        cancel_all_orders(ctx.as_mutable()).should_succeed();

        // USER's orders should be gone.
        for id in 1..=2 {
            assert!(
                BIDS.idx
                    .order_id
                    .may_load(&ctx.storage, Uint64::new(id))
                    .unwrap()
                    .is_none()
            );
        }

        // OTHER_USER's order should still exist.
        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(3))
                .unwrap()
                .is_some()
        );

        // OTHER_USER's state should be untouched.
        let other_state = USER_STATES.load(&ctx.storage, OTHER_USER).unwrap();
        assert_eq!(other_state.open_order_count, 1);
        assert_eq!(other_state.reserved_margin, UsdValue::new_int(70));
    }

    #[test]
    fn cancel_all_with_remaining_position() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid(&mut ctx.storage, 1, USER, 10, 100);

        let mut positions = BTreeMap::new();
        positions.insert("perp/btcusd".parse().unwrap(), dummy_position());
        save_user_state(&mut ctx.storage, USER, 1, 100, positions);

        cancel_all_orders(ctx.as_mutable()).should_succeed();

        // Bid removed.
        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(1))
                .unwrap()
                .is_none()
        );

        // User state still exists because of position.
        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(state.open_order_count, 0);
        assert_eq!(state.reserved_margin, UsdValue::ZERO);
        assert!(!state.is_empty());
    }
}
