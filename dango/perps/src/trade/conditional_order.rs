use {
    crate::{
        CONDITIONAL_ABOVE, CONDITIONAL_BELOW, NEXT_ORDER_ID, PARAM, USER_STATES,
        state::ConditionalOrderKey, trade::update_user_state_with,
    },
    anyhow::{anyhow, ensure},
    dango_types::{
        Dimensionless, Quantity, UsdPrice,
        perps::{
            ConditionalOrder, ConditionalOrderId, ConditionalOrderPlaced, ConditionalOrderRemoved,
            PairId, ReasonForOrderRemoval, TriggerDirection, UserState,
        },
    },
    grug::{
        Addr, EventBuilder, MutableCtx, NumberConst, Order as IterationOrder, Response, StdResult,
        Storage,
    },
};

pub fn submit_conditional_order(
    ctx: MutableCtx,
    pair_id: PairId,
    size: Quantity,
    trigger_price: UsdPrice,
    trigger_direction: TriggerDirection,
    max_slippage: Dimensionless,
) -> anyhow::Result<Response> {
    let param = PARAM.load(ctx.storage)?;
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    // 1. User must have an open position in this pair.
    let position = user_state
        .positions
        .get(&pair_id)
        .ok_or_else(|| anyhow!("no position in pair {pair_id}"))?;

    // 2. Size sign must oppose the position sign (reduce-only).
    ensure!(
        (size.is_negative() && position.size.is_positive())
            || (size.is_positive() && position.size.is_negative()),
        "size must oppose position direction"
    );

    // 3. |size| must not exceed |position.size|.
    let abs_size = size.checked_abs()?;
    let abs_pos_size = position.size.checked_abs()?;
    ensure!(
        abs_size <= abs_pos_size,
        "conditional order size exceeds position size"
    );

    // 4. Must not exceed max conditional orders.
    ensure!(
        user_state.conditional_order_count < param.max_conditional_orders,
        "maximum conditional orders reached"
    );

    // Allocate ID from shared counter.
    let order_id = NEXT_ORDER_ID.load(ctx.storage)?;
    NEXT_ORDER_ID.save(ctx.storage, &(order_id + ConditionalOrderId::ONE))?;

    let order = ConditionalOrder {
        user: ctx.sender,
        size,
        trigger_price,
        trigger_direction,
        max_slippage,
    };

    // Store based on trigger direction.
    let key = (pair_id.clone(), trigger_price, order_id);
    match trigger_direction {
        TriggerDirection::Above => CONDITIONAL_ABOVE.save(ctx.storage, key, &order)?,
        TriggerDirection::Below => CONDITIONAL_BELOW.save(ctx.storage, key, &order)?,
    }

    user_state.conditional_order_count += 1;
    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    Ok(Response::new().add_event(ConditionalOrderPlaced {
        order_id,
        pair_id,
        user: ctx.sender,
        trigger_price,
        trigger_direction,
        size,
        max_slippage,
    })?)
}

pub fn cancel_one_conditional_order(
    ctx: MutableCtx,
    order_id: ConditionalOrderId,
) -> anyhow::Result<Response> {
    // Try CONDITIONAL_ABOVE first, then CONDITIONAL_BELOW (same pattern
    // as cancel_one_order tries BIDS then ASKS).
    let (key, order, is_above) = CONDITIONAL_ABOVE
        .idx
        .order_id
        .may_load(ctx.storage, order_id)?
        .map(|(k, o)| (k, o, true))
        .or_else(|| {
            CONDITIONAL_BELOW
                .idx
                .order_id
                .may_load(ctx.storage, order_id)
                .ok()
                .flatten()
                .map(|(k, o)| (k, o, false))
        })
        .ok_or_else(|| anyhow!("conditional order not found with id {order_id}"))?;

    ensure!(
        ctx.sender == order.user,
        "you are not the owner of this conditional order"
    );

    let event =
        update_user_state_with(ctx.storage, ctx.sender, |storage, user_state| {
            _cancel_one_conditional_order(
                storage,
                user_state,
                key,
                order,
                is_above,
                ReasonForOrderRemoval::Canceled,
            )
        })?;

    Ok(Response::new().add_event(event)?)
}

pub fn cancel_all_conditional_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    let events =
        update_user_state_with(ctx.storage, ctx.sender, |storage, user_state| {
            _cancel_all_conditional_orders(
                storage,
                ctx.sender,
                user_state,
                ReasonForOrderRemoval::Canceled,
            )
        })?;

    Ok(Response::new().add_events(events)?)
}

/// Mutates user_state in memory (decrements count). Does NOT persist it.
/// Removes order from CONDITIONAL_ABOVE or CONDITIONAL_BELOW.
/// Returns the ConditionalOrderRemoved event.
fn _cancel_one_conditional_order(
    storage: &mut dyn Storage,
    user_state: &mut UserState,
    key: ConditionalOrderKey,
    order: ConditionalOrder,
    is_above: bool,
    reason: ReasonForOrderRemoval,
) -> StdResult<ConditionalOrderRemoved> {
    let (pair_id, _, order_id) = &key;
    let event = ConditionalOrderRemoved {
        order_id: *order_id,
        pair_id: pair_id.clone(),
        user: order.user,
        reason,
    };

    if is_above {
        CONDITIONAL_ABOVE.remove(storage, key)?;
    } else {
        CONDITIONAL_BELOW.remove(storage, key)?;
    }

    user_state.conditional_order_count -= 1;

    Ok(event)
}

/// Cancel all conditional orders for a user, updating the in-memory `user_state`.
///
/// Writes to `CONDITIONAL_ABOVE` / `CONDITIONAL_BELOW` in storage but does
/// **not** persist `user_state` — the caller is responsible for saving or
/// removing it.
pub fn _cancel_all_conditional_orders(
    storage: &mut dyn Storage,
    user: Addr,
    user_state: &mut UserState,
    reason: ReasonForOrderRemoval,
) -> StdResult<EventBuilder> {
    let mut events = EventBuilder::new();

    // Collect from both maps.
    let above = CONDITIONAL_ABOVE
        .idx
        .user
        .prefix(user)
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| res.map(|(k, o)| (k, o, true)))
        .collect::<StdResult<Vec<_>>>()?;

    let below = CONDITIONAL_BELOW
        .idx
        .user
        .prefix(user)
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| res.map(|(k, o)| (k, o, false)))
        .collect::<StdResult<Vec<_>>>()?;

    for (key, order, is_above) in above.into_iter().chain(below) {
        events.push(_cancel_one_conditional_order(
            storage, user_state, key, order, is_above, reason,
        )?)?;
    }

    Ok(events)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{CONDITIONAL_ABOVE, CONDITIONAL_BELOW, NEXT_ORDER_ID, PARAM, USER_STATES},
        dango_types::{
            Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue,
            perps::{OrderId, Param, Position, TriggerDirection, UserState},
        },
        grug::{Addr, Coins, MockContext, NumberConst, ResultExt, Storage, Uint64},
        std::collections::BTreeMap,
    };

    const USER: Addr = Addr::mock(1);
    const OTHER_USER: Addr = Addr::mock(2);

    fn pair_id() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    fn long_position(size: i128) -> Position {
        Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(2_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        }
    }

    fn short_position(size: i128) -> Position {
        Position {
            size: Quantity::new_int(-size),
            entry_price: UsdPrice::new_int(2_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        }
    }

    fn user_state_with_position(position: Position) -> UserState {
        let mut positions = BTreeMap::new();
        positions.insert(pair_id(), position);
        UserState {
            margin: UsdValue::new_int(10_000),
            positions,
            ..Default::default()
        }
    }

    fn init_storage(storage: &mut dyn Storage, user_state: UserState) {
        PARAM
            .save(storage, &Param {
                max_conditional_orders: 2,
                ..Default::default()
            })
            .unwrap();
        NEXT_ORDER_ID.save(storage, &OrderId::ONE).unwrap();
        USER_STATES.save(storage, USER, &user_state).unwrap();
    }

    // ======================== placement tests ========================

    #[test]
    fn p1_valid_tp_on_long() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(long_position(10)),
        );

        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-5),
            UsdPrice::new_int(2_500),
            TriggerDirection::Above,
            Dimensionless::new_percent(1),
        )
        .should_succeed();

        // Order stored in CONDITIONAL_ABOVE.
        let order = CONDITIONAL_ABOVE
            .idx
            .order_id
            .may_load(&ctx.storage, Uint64::ONE)
            .unwrap();
        assert!(order.is_some());

        // User state updated.
        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(state.conditional_order_count, 1);

        // Next order ID incremented.
        let next_id = NEXT_ORDER_ID.load(&ctx.storage).unwrap();
        assert_eq!(next_id, Uint64::new(2));
    }

    #[test]
    fn p2_valid_sl_on_long() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(long_position(10)),
        );

        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-10),
            UsdPrice::new_int(1_800),
            TriggerDirection::Below,
            Dimensionless::new_percent(2),
        )
        .should_succeed();

        let order = CONDITIONAL_BELOW
            .idx
            .order_id
            .may_load(&ctx.storage, Uint64::ONE)
            .unwrap();
        assert!(order.is_some());
    }

    #[test]
    fn p3_valid_tp_on_short() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(short_position(10)),
        );

        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(5),
            UsdPrice::new_int(1_500),
            TriggerDirection::Below,
            Dimensionless::new_percent(1),
        )
        .should_succeed();

        let order = CONDITIONAL_BELOW
            .idx
            .order_id
            .may_load(&ctx.storage, Uint64::ONE)
            .unwrap();
        assert!(order.is_some());
    }

    #[test]
    fn p4_reject_wrong_direction() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(long_position(10)),
        );

        // Positive size on a long position = same direction.
        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(5),
            UsdPrice::new_int(2_500),
            TriggerDirection::Above,
            Dimensionless::new_percent(1),
        )
        .should_fail_with_error("size must oppose position direction");
    }

    #[test]
    fn p5_reject_exceeds_position() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(&mut ctx.storage, user_state_with_position(long_position(3)));

        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-5),
            UsdPrice::new_int(2_500),
            TriggerDirection::Above,
            Dimensionless::new_percent(1),
        )
        .should_fail_with_error("exceeds position size");
    }

    #[test]
    fn p6_reject_no_position() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        // User state with no positions.
        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            ..Default::default()
        };
        init_storage(&mut ctx.storage, user_state);

        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-5),
            UsdPrice::new_int(2_500),
            TriggerDirection::Above,
            Dimensionless::new_percent(1),
        )
        .should_fail_with_error("no position");
    }

    #[test]
    fn p7_reject_max_count() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        let mut us = user_state_with_position(long_position(10));
        us.conditional_order_count = 2; // already at max (max_conditional_orders=2)
        init_storage(&mut ctx.storage, us);

        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-5),
            UsdPrice::new_int(2_500),
            TriggerDirection::Above,
            Dimensionless::new_percent(1),
        )
        .should_fail_with_error("maximum conditional orders");
    }

    #[test]
    fn p8_multiple_on_same_pair() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(long_position(10)),
        );

        // TP @ $2,500
        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-5),
            UsdPrice::new_int(2_500),
            TriggerDirection::Above,
            Dimensionless::new_percent(1),
        )
        .should_succeed();

        // SL @ $1,800
        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-10),
            UsdPrice::new_int(1_800),
            TriggerDirection::Below,
            Dimensionless::new_percent(2),
        )
        .should_succeed();

        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(state.conditional_order_count, 2);
    }

    // ======================== cancel tests ========================

    #[test]
    fn c1_cancel_own_order() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        let mut us = user_state_with_position(long_position(10));
        us.conditional_order_count = 1;
        init_storage(&mut ctx.storage, us);

        // Manually store an order.
        let order = ConditionalOrder {
            user: USER,
            size: Quantity::new_int(-5),
            trigger_price: UsdPrice::new_int(2_500),
            trigger_direction: TriggerDirection::Above,
            max_slippage: Dimensionless::new_percent(1),
        };
        let key = (pair_id(), UsdPrice::new_int(2_500), Uint64::new(7));
        CONDITIONAL_ABOVE
            .save(&mut ctx.storage, key, &order)
            .unwrap();

        cancel_one_conditional_order(ctx.as_mutable(), Uint64::new(7)).should_succeed();

        // Order removed.
        assert!(
            CONDITIONAL_ABOVE
                .idx
                .order_id
                .may_load(&ctx.storage, Uint64::new(7))
                .unwrap()
                .is_none()
        );

        // Count decremented.
        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(state.conditional_order_count, 0);
    }

    #[test]
    fn c2_reject_not_owner() {
        let mut ctx = MockContext::new()
            .with_sender(OTHER_USER)
            .with_funds(Coins::default());

        // Store an order owned by USER.
        let mut us = user_state_with_position(long_position(10));
        us.conditional_order_count = 1;
        PARAM
            .save(&mut ctx.storage, &Param {
                max_conditional_orders: 2,
                ..Default::default()
            })
            .unwrap();
        NEXT_ORDER_ID.save(&mut ctx.storage, &OrderId::ONE).unwrap();
        USER_STATES.save(&mut ctx.storage, USER, &us).unwrap();
        USER_STATES
            .save(&mut ctx.storage, OTHER_USER, &UserState {
                margin: UsdValue::new_int(1_000),
                ..Default::default()
            })
            .unwrap();

        let order = ConditionalOrder {
            user: USER,
            size: Quantity::new_int(-5),
            trigger_price: UsdPrice::new_int(2_500),
            trigger_direction: TriggerDirection::Above,
            max_slippage: Dimensionless::new_percent(1),
        };
        let key = (pair_id(), UsdPrice::new_int(2_500), Uint64::new(7));
        CONDITIONAL_ABOVE
            .save(&mut ctx.storage, key, &order)
            .unwrap();

        cancel_one_conditional_order(ctx.as_mutable(), Uint64::new(7))
            .should_fail_with_error("not the owner");
    }

    #[test]
    fn c3_reject_nonexistent() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(long_position(10)),
        );

        cancel_one_conditional_order(ctx.as_mutable(), Uint64::new(99))
            .should_fail_with_error("not found");
    }

    #[test]
    fn c4_cancel_all() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        let mut us = user_state_with_position(long_position(10));
        us.conditional_order_count = 3;
        init_storage(&mut ctx.storage, us);

        // Store 3 orders across both maps.
        for (id, price, is_above) in [(1u64, 2_500i128, true), (2, 1_800, false), (3, 3_000, true)]
        {
            let order = ConditionalOrder {
                user: USER,
                size: Quantity::new_int(-5),
                trigger_price: UsdPrice::new_int(price),
                trigger_direction: if is_above {
                    TriggerDirection::Above
                } else {
                    TriggerDirection::Below
                },
                max_slippage: Dimensionless::new_percent(1),
            };
            let key = (pair_id(), UsdPrice::new_int(price), Uint64::new(id));
            if is_above {
                CONDITIONAL_ABOVE
                    .save(&mut ctx.storage, key, &order)
                    .unwrap();
            } else {
                CONDITIONAL_BELOW
                    .save(&mut ctx.storage, key, &order)
                    .unwrap();
            }
        }

        cancel_all_conditional_orders(ctx.as_mutable()).should_succeed();

        // All removed.
        for id in 1..=3u64 {
            assert!(
                CONDITIONAL_ABOVE
                    .idx
                    .order_id
                    .may_load(&ctx.storage, Uint64::new(id))
                    .unwrap()
                    .is_none()
            );
            assert!(
                CONDITIONAL_BELOW
                    .idx
                    .order_id
                    .may_load(&ctx.storage, Uint64::new(id))
                    .unwrap()
                    .is_none()
            );
        }

        // Count = 0.
        let state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(state.conditional_order_count, 0);
    }

    #[test]
    fn c5_cancel_all_no_orders() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(long_position(10)),
        );

        // No conditional orders exist — should succeed with no changes.
        cancel_all_conditional_orders(ctx.as_mutable()).should_succeed();
    }
}
