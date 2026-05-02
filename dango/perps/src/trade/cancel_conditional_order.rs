use {
    crate::USER_STATES,
    anyhow::anyhow,
    dango_order_book::{ConditionalOrderRemoved, PairId, ReasonForOrderRemoval, TriggerDirection},
    grug::{MutableCtx, Response},
};

pub fn cancel_one_conditional_order(
    ctx: MutableCtx,
    pair_id: PairId,
    trigger_direction: TriggerDirection,
) -> anyhow::Result<Response> {
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    let position = user_state
        .positions
        .get_mut(&pair_id)
        .ok_or_else(|| anyhow!("no position in pair {pair_id}"))?;

    let _order = match trigger_direction {
        TriggerDirection::Above => position
            .conditional_order_above
            .take()
            .ok_or_else(|| anyhow!("no conditional order above for pair {pair_id}"))?,
        TriggerDirection::Below => position
            .conditional_order_below
            .take()
            .ok_or_else(|| anyhow!("no conditional order below for pair {pair_id}"))?,
    };

    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    Ok(Response::new().add_event(ConditionalOrderRemoved {
        pair_id,
        user: ctx.sender,
        trigger_direction,
        reason: ReasonForOrderRemoval::Canceled,
    })?)
}

pub fn cancel_conditional_orders_for_pair(
    ctx: MutableCtx,
    pair_id: PairId,
) -> anyhow::Result<Response> {
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    let position = user_state
        .positions
        .get_mut(&pair_id)
        .ok_or_else(|| anyhow!("no position in pair {pair_id}"))?;

    let had_above = position.conditional_order_above.take().is_some();
    let had_below = position.conditional_order_below.take().is_some();

    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    let mut response = Response::new();

    if had_above {
        response = response.add_event(ConditionalOrderRemoved {
            pair_id: pair_id.clone(),
            user: ctx.sender,
            trigger_direction: TriggerDirection::Above,
            reason: ReasonForOrderRemoval::Canceled,
        })?;
    }

    if had_below {
        response = response.add_event(ConditionalOrderRemoved {
            pair_id,
            user: ctx.sender,
            trigger_direction: TriggerDirection::Below,
            reason: ReasonForOrderRemoval::Canceled,
        })?;
    }

    Ok(response)
}

pub fn cancel_all_conditional_orders(ctx: MutableCtx) -> anyhow::Result<Response> {
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    let mut response = Response::new();

    for (pair_id, position) in &mut user_state.positions {
        if let Some(_order) = position.conditional_order_above.take() {
            response = response.add_event(ConditionalOrderRemoved {
                pair_id: pair_id.clone(),
                user: ctx.sender,
                trigger_direction: TriggerDirection::Above,
                reason: ReasonForOrderRemoval::Canceled,
            })?;
        }

        if let Some(_order) = position.conditional_order_below.take() {
            response = response.add_event(ConditionalOrderRemoved {
                pair_id: pair_id.clone(),
                user: ctx.sender,
                trigger_direction: TriggerDirection::Below,
                reason: ReasonForOrderRemoval::Canceled,
            })?;
        }
    }

    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    Ok(response)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{NEXT_ORDER_ID, PARAM, USER_STATES},
        dango_order_book::{
            ConditionalOrder, Dimensionless, FundingPerUnit, OrderId, PairId, Quantity,
            TriggerDirection, UsdPrice, UsdValue,
        },
        dango_types::perps::{Param, Position, UserState},
        grug::{Addr, Coins, MockContext, NumberConst, ResultExt, Storage, Uint64},
        std::collections::BTreeMap,
    };

    const USER: Addr = Addr::mock(1);

    fn pair_id() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    fn pair_id_2() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn long_position(size: i128) -> Position {
        Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(2_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        }
    }

    fn long_position_with_orders(
        size: i128,
        above: Option<ConditionalOrder>,
        below: Option<ConditionalOrder>,
    ) -> Position {
        Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(2_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: above,
            conditional_order_below: below,
        }
    }

    fn make_conditional_order(order_id: u64, size: i128, trigger_price: i128) -> ConditionalOrder {
        ConditionalOrder {
            order_id: Uint64::new(order_id),
            size: Some(Quantity::new_int(size)),
            trigger_price: UsdPrice::new_int(trigger_price),
            max_slippage: Dimensionless::new_percent(1),
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
        PARAM.save(storage, &Param::default()).unwrap();
        NEXT_ORDER_ID.save(storage, &OrderId::ONE).unwrap();
        USER_STATES.save(storage, USER, &user_state).unwrap();
    }

    #[test]
    fn c1_cancel_one_above() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        let position =
            long_position_with_orders(10, Some(make_conditional_order(1, -5, 2_500)), None);
        init_storage(&mut ctx.storage, user_state_with_position(position));

        cancel_one_conditional_order(ctx.as_mutable(), pair_id(), TriggerDirection::Above)
            .should_succeed();

        // Order removed from position.
        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let position = user_state.positions.get(&pair_id()).unwrap();
        assert!(position.conditional_order_above.is_none());
    }

    #[test]
    fn c2_cancel_one_below() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        let position =
            long_position_with_orders(10, None, Some(make_conditional_order(1, -10, 1_800)));
        init_storage(&mut ctx.storage, user_state_with_position(position));

        cancel_one_conditional_order(ctx.as_mutable(), pair_id(), TriggerDirection::Below)
            .should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let position = user_state.positions.get(&pair_id()).unwrap();
        assert!(position.conditional_order_below.is_none());
    }

    #[test]
    fn c3_reject_no_order() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(long_position(10)),
        );

        cancel_one_conditional_order(ctx.as_mutable(), pair_id(), TriggerDirection::Above)
            .should_fail_with_error("no conditional order above");
    }

    #[test]
    fn c4_cancel_all() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        let position = long_position_with_orders(
            10,
            Some(make_conditional_order(1, -5, 2_500)),
            Some(make_conditional_order(2, -10, 1_800)),
        );
        init_storage(&mut ctx.storage, user_state_with_position(position));

        cancel_all_conditional_orders(ctx.as_mutable()).should_succeed();

        // All orders removed.
        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let position = user_state.positions.get(&pair_id()).unwrap();
        assert!(position.conditional_order_above.is_none());
        assert!(position.conditional_order_below.is_none());
    }

    #[test]
    fn c5_cancel_for_pair() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        // User has positions in two pairs, each with conditional orders.
        let mut positions = BTreeMap::new();
        positions.insert(
            pair_id(),
            long_position_with_orders(
                10,
                Some(make_conditional_order(1, -5, 2_500)),
                Some(make_conditional_order(2, -10, 1_800)),
            ),
        );
        positions.insert(
            pair_id_2(),
            long_position_with_orders(5, Some(make_conditional_order(3, -3, 50_000)), None),
        );

        let user_state = UserState {
            margin: UsdValue::new_int(10_000),
            positions,
            ..Default::default()
        };
        init_storage(&mut ctx.storage, user_state);

        // Cancel only pair_id's orders.
        cancel_conditional_orders_for_pair(ctx.as_mutable(), pair_id()).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();

        // pair_id orders cleared.
        let position = user_state.positions.get(&pair_id()).unwrap();
        assert!(position.conditional_order_above.is_none());
        assert!(position.conditional_order_below.is_none());

        // pair_id_2 order still exists.
        let position_2 = user_state.positions.get(&pair_id_2()).unwrap();
        assert!(position_2.conditional_order_above.is_some());
        assert_eq!(
            position_2
                .conditional_order_above
                .as_ref()
                .unwrap()
                .order_id,
            Uint64::new(3)
        );
    }

    #[test]
    fn c6_cancel_all_no_orders() {
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
