use {
    crate::{ASKS, BIDS, USER_STATES},
    anyhow::{anyhow, ensure},
    dango_types::perps::OrderId,
    grug::{MutableCtx, Order as IterationOrder, Response, StdResult},
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

    // Delete the order.
    if order.size.is_positive() {
        BIDS.remove(ctx.storage, order_key)?;
    } else {
        ASKS.remove(ctx.storage, order_key)?;
    }

    // Update user state: release reserved margin and decrement open order count.
    USER_STATES.modify(ctx.storage, ctx.sender, |mut user_state| -> StdResult<_> {
        user_state.open_order_count -= 1;
        (user_state.reserved_margin).checked_sub_assign(order.reserved_margin)?;

        // Delete the user state if it's empty. Otherwise, save the updated user state.
        if user_state.is_empty() {
            Ok(None)
        } else {
            Ok(Some(user_state))
        }
    })?;

    Ok(Response::new())
}

pub fn cancel_all_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    // Load the sender's user state.
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    // For bids and asks respectively, first collect all orders into memory;
    // then for each order, 1) delete it, 2) release reserved margin and decrement
    // open order count.
    for map in [BIDS, ASKS] {
        for (order_key, order) in map
            .idx
            .user
            .prefix(ctx.sender)
            .range(ctx.storage, None, None, IterationOrder::Ascending)
            .collect::<StdResult<Vec<_>>>()?
        {
            map.remove(ctx.storage, order_key)?;

            user_state.open_order_count -= 1;
            (user_state.reserved_margin).checked_sub_assign(order.reserved_margin)?;
        }
    }

    // Delete the user state if it's empty. Otherwise, save the updated user state.
    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.sender);
    } else {
        USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    }

    Ok(Response::new())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            USER_STATES,
            state::{ASKS, BIDS, OrderKey},
        },
        dango_types::{
            FundingPerUnit, Quantity, UsdPrice, UsdValue,
            perps::{Order, Position, UserState},
        },
        grug::{Addr, Coins, MockContext, ResultExt, Storage, Uint64},
        std::collections::{BTreeMap, VecDeque},
    };

    const USER: Addr = Addr::mock(1);
    const OTHER_USER: Addr = Addr::mock(2);

    /// Build an `OrderKey` with fixed defaults for pair, price, and timestamp.
    fn order_key(order_id: u64) -> OrderKey {
        (
            "perp/btcusd".parse().unwrap(),
            UsdPrice::new_int(50_000),
            Uint64::new(order_id),
        )
    }

    /// Save a bid (positive size) into `BIDS`.
    fn save_bid(
        storage: &mut dyn Storage,
        order_id: u64,
        user: Addr,
        size: i128,
        reserved_margin: i128,
    ) {
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
