use {
    crate::{state::PAIR_PARAMS, trade::update_user_state_with},
    anyhow::{anyhow, ensure},
    dango_order_book::{
        ASKS, BIDS, ClientOrderId, LimitOrder, OrderId, OrderKey, PairId, ReasonForOrderRemoval,
        remove_order,
    },
    dango_types::perps::{PairParam, UserState},
    grug::{Addr, EventBuilder, MutableCtx, Order as IterationOrder, Response, StdResult, Storage},
    std::collections::{BTreeMap, BTreeSet},
};

pub fn cancel_one_order(ctx: MutableCtx, order_id: OrderId) -> anyhow::Result<Response> {
    let mut events = EventBuilder::new();
    _cancel_one_order(ctx.storage, ctx.sender, order_id, &mut events)?;
    Ok(Response::new().add_events(events)?)
}

/// Intermediate layer of `cancel_one_order`: takes individual components
/// of `MutableCtx` so multiple invocations (e.g. inside
/// `batch_update_orders`) can share the same storage. Pushes events into
/// the caller-owned builder; the caller assembles the `Response`.
pub(crate) fn _cancel_one_order(
    storage: &mut dyn Storage,
    sender: Addr,
    order_id: OrderId,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    // Since we don't know whether it's a buy or a sell order, we first attempt
    // to load it from the `BIDS` map. If not found, load it from `ASKS`.
    // If still not found, bail.
    let (order_key, order) = BIDS
        .idx
        .order_id
        .may_load(storage, order_id)
        .transpose()
        .or_else(|| ASKS.idx.order_id.may_load(storage, order_id).transpose())
        .ok_or_else(|| anyhow!("order not found with id {order_id}"))??;

    ensure!(sender == order.user, "you are not the owner of this order");

    update_user_state_with(storage, sender, |storage, user_state| {
        compute_cancel_one_order_outcome(
            storage,
            user_state,
            order_key,
            order,
            Some(events),
            ReasonForOrderRemoval::Canceled,
            |storage, pair_id| PAIR_PARAMS.load(storage, pair_id),
        )
    })?;

    Ok(())
}

/// Mutates:
///
/// - User state: releases reserved margin, decrement open order count.
///   Does NOT save updated user state to storage. Closing this one order may be
///   only step in a bigger routine -- the caller may want to do other changes
///   to the user state before saving.
/// - Remove the order from the `BIDS` or `ASKS` map.
/// - Remove liquidity depth contributed by this order.
fn compute_cancel_one_order_outcome<F>(
    storage: &mut dyn Storage,
    user_state: &mut UserState,
    order_key: OrderKey,
    order: LimitOrder,
    events: Option<&mut EventBuilder>,
    reason: ReasonForOrderRemoval,
    pair_param: F,
) -> StdResult<()>
where
    F: FnOnce(&dyn Storage, &PairId) -> StdResult<PairParam>,
{
    let (pair_id, ..) = &order_key;
    let pair_param = pair_param(storage, pair_id)?;

    // Perp-side: release reserved margin and decrement the user's
    // open-order count. The generic order-book primitive below handles
    // the rest (depth decrement, removing the entry from BIDS/ASKS,
    // emitting `OrderRemoved`).
    (user_state.reserved_margin).checked_sub_assign(order.reserved_margin)?;
    user_state.open_order_count -= 1;

    remove_order(
        storage,
        order_key,
        &order,
        reason,
        &pair_param.bucket_sizes,
        events,
    )?;

    Ok(())
}

/// Cancel one of the sender's resting limit orders, looked up by the
/// `ClientOrderId` the sender assigned at submission time.
///
/// The `(sender, client_order_id)` index on `BIDS` / `ASKS` is consulted
/// — that index is per-sender, so the lookup can only ever return orders
/// owned by `ctx.sender`, which means no redundant ownership check is
/// needed. The actual cancellation delegates to [`compute_cancel_one_order_outcome`]
/// with the loaded `(order_key, order)` so we don't re-resolve through
/// the `order_id` index.
pub fn cancel_one_order_by_client_order_id(
    ctx: MutableCtx,
    client_order_id: ClientOrderId,
) -> anyhow::Result<Response> {
    let mut events = EventBuilder::new();
    _cancel_one_order_by_client_order_id(ctx.storage, ctx.sender, client_order_id, &mut events)?;
    Ok(Response::new().add_events(events)?)
}

/// Intermediate layer of `cancel_one_order_by_client_order_id`.
pub(crate) fn _cancel_one_order_by_client_order_id(
    storage: &mut dyn Storage,
    sender: Addr,
    client_order_id: ClientOrderId,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    let key = (sender, client_order_id);

    // `or_else` is lazy: ASKS is only consulted on a BIDS miss.
    let (order_key, order) = BIDS
        .idx
        .client_order_id
        .may_load(storage, key)
        .transpose()
        .or_else(|| ASKS.idx.client_order_id.may_load(storage, key).transpose())
        .ok_or_else(|| {
            anyhow!("order not found with user {sender} and client_order_id {client_order_id}")
        })??;

    update_user_state_with(storage, sender, |storage, user_state| {
        compute_cancel_one_order_outcome(
            storage,
            user_state,
            order_key,
            order,
            Some(events),
            ReasonForOrderRemoval::Canceled,
            |storage, pair_id| PAIR_PARAMS.load(storage, pair_id),
        )
    })?;

    Ok(())
}

pub fn cancel_all_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    let mut events = EventBuilder::new();
    _cancel_all_orders(ctx.storage, ctx.sender, &mut events)?;
    Ok(Response::new().add_events(events)?)
}

/// Intermediate layer of `cancel_all_orders`.
pub(crate) fn _cancel_all_orders(
    storage: &mut dyn Storage,
    sender: Addr,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    update_user_state_with(storage, sender, |storage, user_state| {
        let CancelAllOrdersOutcome {
            user_state: updated_user_state,
        } = compute_cancel_all_orders_outcome(
            storage,
            sender,
            user_state,
            Some(events),
            ReasonForOrderRemoval::Canceled,
        )?;

        *user_state = updated_user_state;

        Ok(())
    })?;

    Ok(())
}

/// Owned outcome of a `compute_cancel_all_orders_outcome` call. Carries only the
/// updated `user_state` — storage mutations (`BIDS` / `ASKS` removal,
/// liquidity-depth decrement) happen *inside* the function because
/// storage has tx-level rollback at the block boundary and there is no
/// point deferring them.
#[derive(Debug)]
pub struct CancelAllOrdersOutcome {
    pub user_state: UserState,
}

/// Cancel all resting orders for a user, returning the updated
/// `UserState` in a [`CancelAllOrdersOutcome`]. Writes to `BIDS` / `ASKS`
/// and the liquidity-depth maps happen inline inside the function —
/// storage has tx-level rollback, so they don't need to be deferred.
///
/// Pure w.r.t. the caller's `UserState`: takes `&UserState`, clones
/// internally, and returns the updated copy in the outcome. The
/// caller is responsible for saving or removing
/// `outcome.user_state`. See `dango/perps/purity.md` for the full
/// rationale.
pub fn compute_cancel_all_orders_outcome(
    storage: &mut dyn Storage,
    user: Addr,
    user_state: &UserState,
    mut events: Option<&mut EventBuilder>,
    reason: ReasonForOrderRemoval,
) -> StdResult<CancelAllOrdersOutcome> {
    // Clone the user state and mutate the local copy. On `Err` the clone
    // is dropped with the rest of the call frame; the caller's
    // `&UserState` is never touched.
    let mut user_state = user_state.clone();

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
        compute_cancel_one_order_outcome(
            storage,
            &mut user_state,
            order_key,
            order,
            events.as_deref_mut(),
            reason,
            |_, pair_id| Ok(pair_params[pair_id].clone()),
        )?;
    }

    Ok(CancelAllOrdersOutcome { user_state })
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::state::{PAIR_PARAMS, USER_STATES},
        dango_order_book::{
            ASKS, BIDS, FundingPerUnit, LimitOrder, OrderKey, OrderRemoved, PairId, Quantity,
            UsdPrice, UsdValue,
        },
        dango_types::perps::{PairParam, Position, UserState},
        grug::{
            Addr, Coins, EventName, JsonDeExt, MockContext, ResultExt, Storage, Timestamp, Uint64,
        },
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
        let order = LimitOrder {
            user,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(reserved_margin),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
            client_order_id: None,
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
        let order = LimitOrder {
            user,
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

    /// Save a bid carrying a `client_order_id` so the
    /// `cancel_one_order_by_client_order_id` tests can resolve it.
    fn save_bid_with_cid(
        storage: &mut dyn Storage,
        order_id: u64,
        user: Addr,
        size: i128,
        reserved_margin: i128,
        cid: u64,
    ) {
        ensure_pair_param(storage);
        let key = order_key(order_id);
        let order = LimitOrder {
            user,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(reserved_margin),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
            client_order_id: Some(Uint64::new(cid)),
        };

        BIDS.save(storage, key, &order).unwrap();
    }

    /// Same as [`save_bid_with_cid`] but for asks.
    fn save_ask_with_cid(
        storage: &mut dyn Storage,
        order_id: u64,
        user: Addr,
        size: i128,
        reserved_margin: i128,
        cid: u64,
    ) {
        ensure_pair_param(storage);
        let key = order_key(order_id);
        let order = LimitOrder {
            user,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(reserved_margin),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
            client_order_id: Some(Uint64::new(cid)),
        };

        ASKS.save(storage, key, &order).unwrap();
    }

    /// Save a `UserState` with the given order count and reserved margin.
    fn save_user_state(
        storage: &mut dyn Storage,
        user: Addr,
        open_order_count: usize,
        reserved_margin: i128,
        positions: BTreeMap<PairId, Position>,
    ) {
        let state = UserState {
            unlocks: VecDeque::new(),
            positions,
            reserved_margin: UsdValue::new_int(reserved_margin),
            open_order_count,
            ..Default::default()
        };

        USER_STATES.save(storage, user, &state).unwrap();
    }

    /// Create a dummy position for testing "user state not empty" scenarios.
    fn dummy_position() -> Position {
        Position {
            size: Quantity::new_int(1),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
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

    // ============== cancel_one_order_by_client_order_id tests ====================

    #[test]
    fn cancel_one_order_by_client_order_id_bid() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid_with_cid(&mut ctx.storage, 1, USER, 10, 100, 7);
        save_user_state(&mut ctx.storage, USER, 1, 100, BTreeMap::new());

        cancel_one_order_by_client_order_id(ctx.as_mutable(), Uint64::new(7)).should_succeed();

        // Primary entry gone.
        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(1))
                .unwrap()
                .is_none()
        );

        // Index entry gone — the same `cid` is reusable.
        assert!(
            BIDS.idx
                .client_order_id
                .may_load_key(&ctx.storage, (USER, Uint64::new(7)))
                .unwrap()
                .is_none()
        );

        // User state cleaned up (was empty).
        assert!(USER_STATES.may_load(&ctx.storage, USER).unwrap().is_none());
    }

    #[test]
    fn cancel_one_order_by_client_order_id_ask() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_ask_with_cid(&mut ctx.storage, 1, USER, -10, 100, 7);
        save_user_state(&mut ctx.storage, USER, 1, 100, BTreeMap::new());

        cancel_one_order_by_client_order_id(ctx.as_mutable(), Uint64::new(7)).should_succeed();

        assert!(
            ASKS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(1))
                .unwrap()
                .is_none()
        );

        assert!(
            ASKS.idx
                .client_order_id
                .may_load_key(&ctx.storage, (USER, Uint64::new(7)))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn cancel_one_order_by_client_order_id_not_found() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        cancel_one_order_by_client_order_id(ctx.as_mutable(), Uint64::new(99))
            .should_fail_with_error("order not found");
    }

    /// `OrderRemoved` events emitted by `cancel_one_order` carry the
    /// resting order's `client_order_id`, so off-chain consumers can
    /// correlate the cancellation with the originally-submitted cid.
    #[test]
    fn cancel_emits_order_removed_with_client_order_id() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        save_bid_with_cid(&mut ctx.storage, 1, USER, 10, 100, 7);
        save_user_state(&mut ctx.storage, USER, 1, 100, BTreeMap::new());

        let response = cancel_one_order(ctx.as_mutable(), Uint64::new(1)).unwrap();
        let event = response
            .subevents
            .iter()
            .find(|e| e.ty == OrderRemoved::EVENT_NAME)
            .expect("OrderRemoved event missing");
        let order_removed: OrderRemoved = event.data.clone().deserialize_json().unwrap();
        assert_eq!(order_removed.order_id, Uint64::new(1));
        assert_eq!(order_removed.user, USER);
        assert_eq!(order_removed.client_order_id, Some(Uint64::new(7)));
    }

    /// Two different users can independently use the same `client_order_id`
    /// value; each can only cancel their own.
    #[test]
    fn cancel_one_order_by_client_order_id_other_user_isolated() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        // OTHER_USER also uses cid=7, on a separate order id.
        save_bid_with_cid(&mut ctx.storage, 1, USER, 10, 100, 7);
        save_bid_with_cid(&mut ctx.storage, 2, OTHER_USER, 5, 50, 7);
        save_user_state(&mut ctx.storage, USER, 1, 100, BTreeMap::new());
        save_user_state(&mut ctx.storage, OTHER_USER, 1, 50, BTreeMap::new());

        // USER cancels by cid=7 — only USER's order should disappear.
        cancel_one_order_by_client_order_id(ctx.as_mutable(), Uint64::new(7)).should_succeed();

        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(1))
                .unwrap()
                .is_none()
        );
        // OTHER_USER's order with the same cid is untouched.
        assert!(
            BIDS.idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(2))
                .unwrap()
                .is_some()
        );
        assert!(
            BIDS.idx
                .client_order_id
                .may_load_key(&ctx.storage, (OTHER_USER, Uint64::new(7)))
                .unwrap()
                .is_some()
        );
    }
}
