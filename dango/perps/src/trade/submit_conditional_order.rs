use {
    crate::{NEXT_ORDER_ID, USER_STATES},
    anyhow::{anyhow, ensure},
    dango_types::{
        Dimensionless, Quantity, UsdPrice,
        perps::{
            ConditionalOrder, ConditionalOrderId, ConditionalOrderPlaced, PairId, TriggerDirection,
        },
    },
    grug::{MutableCtx, NumberConst, Response},
};

pub fn submit_conditional_order(
    ctx: MutableCtx,
    pair_id: PairId,
    size: Quantity,
    trigger_price: UsdPrice,
    trigger_direction: TriggerDirection,
    max_slippage: Dimensionless,
) -> anyhow::Result<Response> {
    let mut user_state = USER_STATES.load(ctx.storage, ctx.sender)?;

    // -------------------------------- Checks ---------------------------------

    // 1. User must have an open position in this pair.
    // 2. Size sign must oppose the position sign (reduce-only).
    // 3. |size| must not exceed |position.size|.
    // 4. Must not already have a conditional order of the same direction for this pair.

    let position = user_state
        .positions
        .get(&pair_id)
        .ok_or_else(|| anyhow!("no position in pair {pair_id}"))?;

    ensure!(
        (size.is_negative() && position.size.is_positive())
            || (size.is_positive() && position.size.is_negative()),
        "size must oppose position direction"
    );

    ensure!(
        {
            let abs_size = size.checked_abs()?;
            let abs_pos_size = position.size.checked_abs()?;
            abs_size <= abs_pos_size
        },
        "conditional order size exceeds position size"
    );

    let slot_occupied = match trigger_direction {
        TriggerDirection::Above => position.conditional_order_above.is_some(),
        TriggerDirection::Below => position.conditional_order_below.is_some(),
    };

    ensure!(
        !slot_occupied,
        "conditional order already exists for pair {pair_id}"
    );

    // ----------------------------- State changes -----------------------------

    // Assign order ID and increment.
    let order_id = NEXT_ORDER_ID.load(ctx.storage)?;
    NEXT_ORDER_ID.save(ctx.storage, &(order_id + ConditionalOrderId::ONE))?;

    let conditional_order = ConditionalOrder {
        order_id,
        size,
        trigger_price,
        max_slippage,
    };

    // Set the field on the position.
    let position = user_state
        .positions
        .get_mut(&pair_id)
        .ok_or_else(|| anyhow!("no position in pair {pair_id}"))?;

    match trigger_direction {
        TriggerDirection::Above => position.conditional_order_above = Some(conditional_order),
        TriggerDirection::Below => position.conditional_order_below = Some(conditional_order),
    }

    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    Ok(Response::new().add_event(ConditionalOrderPlaced {
        pair_id,
        user: ctx.sender,
        trigger_price,
        trigger_direction,
        size,
        max_slippage,
    })?)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{NEXT_ORDER_ID, PARAM, USER_STATES},
        dango_types::{
            Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue,
            perps::{OrderId, Param, Position, TriggerDirection, UserState},
        },
        grug::{Addr, Coins, MockContext, NumberConst, ResultExt, Storage, Uint64},
        std::collections::BTreeMap,
    };

    const USER: Addr = Addr::mock(1);

    fn pair_id() -> PairId {
        "perp/ethusd".parse().unwrap()
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

    fn short_position(size: i128) -> Position {
        Position {
            size: Quantity::new_int(-size),
            entry_price: UsdPrice::new_int(2_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
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

        // Order stored in position's conditional_order_above field.
        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let position = user_state.positions.get(&pair_id()).unwrap();
        assert!(position.conditional_order_above.is_some());
        let order = position.conditional_order_above.as_ref().unwrap();
        assert_eq!(order.order_id, Uint64::ONE);
        assert_eq!(order.size, Quantity::new_int(-5));
        assert_eq!(order.trigger_price, UsdPrice::new_int(2_500));

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

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let position = user_state.positions.get(&pair_id()).unwrap();
        assert!(position.conditional_order_below.is_some());
        let order = position.conditional_order_below.as_ref().unwrap();
        assert_eq!(order.order_id, Uint64::ONE);
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

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let position = user_state.positions.get(&pair_id()).unwrap();
        assert!(position.conditional_order_below.is_some());
        let order = position.conditional_order_below.as_ref().unwrap();
        assert_eq!(order.order_id, Uint64::ONE);
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
    fn p7_reject_duplicate_direction() {
        let mut ctx = MockContext::new()
            .with_sender(USER)
            .with_funds(Coins::default());

        init_storage(
            &mut ctx.storage,
            user_state_with_position(long_position(10)),
        );

        // First Above order — should succeed.
        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-5),
            UsdPrice::new_int(2_500),
            TriggerDirection::Above,
            Dimensionless::new_percent(1),
        )
        .should_succeed();

        // Second Above order for same pair — should fail.
        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-3),
            UsdPrice::new_int(3_000),
            TriggerDirection::Above,
            Dimensionless::new_percent(1),
        )
        .should_fail_with_error("already exists");

        // Below order for same pair — should succeed (different direction).
        submit_conditional_order(
            ctx.as_mutable(),
            pair_id(),
            Quantity::new_int(-5),
            UsdPrice::new_int(1_800),
            TriggerDirection::Below,
            Dimensionless::new_percent(2),
        )
        .should_succeed();
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

        // Both orders exist on the position.
        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let position = user_state.positions.get(&pair_id()).unwrap();

        assert!(position.conditional_order_above.is_some());
        let above = position.conditional_order_above.as_ref().unwrap();
        assert_eq!(above.order_id, Uint64::ONE);
        assert_eq!(above.trigger_price, UsdPrice::new_int(2_500));

        assert!(position.conditional_order_below.is_some());
        let below = position.conditional_order_below.as_ref().unwrap();
        assert_eq!(below.order_id, Uint64::new(2));
        assert_eq!(below.trigger_price, UsdPrice::new_int(1_800));
    }
}
