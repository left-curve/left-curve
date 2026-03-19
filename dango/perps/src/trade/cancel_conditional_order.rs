use {
    crate::{
        CONDITIONAL_ABOVE, CONDITIONAL_BELOW, state::ConditionalOrderKey,
        trade::update_user_state_with,
    },
    anyhow::{anyhow, ensure},
    dango_types::perps::{
        ConditionalOrder, ConditionalOrderId, ConditionalOrderRemoved, ReasonForOrderRemoval,
        UserState,
    },
    grug::{Addr, EventBuilder, MutableCtx, Order as IterationOrder, Response, StdResult, Storage},
};

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

    let event = update_user_state_with(ctx.storage, ctx.sender, |storage, user_state| {
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

pub fn cancel_all_conditional_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    let events = update_user_state_with(ctx.storage, ctx.sender, |storage, user_state| {
        _cancel_all_conditional_orders(
            storage,
            ctx.sender,
            user_state,
            ReasonForOrderRemoval::Canceled,
        )
    })?;

    Ok(Response::new().add_events(events)?)
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
            perps::{OrderId, PairId, Param, Position, TriggerDirection, UserState},
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
