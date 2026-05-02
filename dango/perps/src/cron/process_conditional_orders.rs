use {
    crate::{
        position_index::apply_position_index_updates,
        referral::{FeeCommissionsOutcome, apply_fee_commissions},
        state::{PAIR_IDS, PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES},
        trade::{SubmitOrderOutcome, compute_submit_order_outcome},
    },
    dango_oracle::OracleQuerier,
    dango_order_book::{
        ConditionalOrderRemoved, ConditionalOrderTriggered, OrderKind, PairId,
        ReasonForOrderRemoval, TriggerDirection, UsdPrice, decrease_liquidity_depths,
        flush_volumes, increase_liquidity_depths, may_invert_price,
        state::{ASKS, BIDS, NEXT_FILL_ID, NEXT_ORDER_ID},
    },
    dango_types::perps::{PairParam, PairState, Param, State},
    grug::{
        Addr, EventBuilder, NumberConst, Order as IterationOrder, PrefixBound, QuerierWrapper,
        StdResult, Storage, Timestamp, Uint64,
    },
};

/// Evaluate and trigger conditional orders whose trigger conditions are met.
///
/// Called from `cron_execute` after `process_funding`. Uses range-bounded
/// iteration so only triggered orders are visited (no full scan).
pub fn process_conditional_orders(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    contract: Addr,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    let param = PARAM.load(storage)?;
    let mut state = STATE.load(storage)?;
    let pair_ids = PAIR_IDS.load(storage)?;

    for pair_id in pair_ids {
        process_conditional_orders_for_pair(
            storage,
            querier,
            contract,
            current_time,
            oracle_querier,
            &param,
            &mut state,
            &pair_id,
            events,
        )?;
    }

    STATE.save(storage, &state)?;

    Ok(())
}

fn process_conditional_orders_for_pair(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    contract: Addr,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    state: &mut State,
    pair_id: &PairId,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;
    let pair_param = PAIR_PARAMS.load(storage, pair_id)?;
    let mut pair_state = PAIR_STATES.load(storage, pair_id)?;

    // ABOVE orders: trigger when oracle_price >= trigger_price.
    // Range: all keys with trigger_price <= oracle_price.
    let above_triggered = USER_STATES
        .idx
        .conditional_orders
        .sub_prefix(pair_id.clone())
        .prefix_range(
            storage,
            Some(PrefixBound::Inclusive((
                TriggerDirection::Above,
                UsdPrice::MIN,
                Uint64::MIN,
            ))),
            Some(PrefixBound::Inclusive((
                TriggerDirection::Above,
                oracle_price,
                Uint64::MAX,
            ))),
            IterationOrder::Ascending,
        )
        .collect::<StdResult<Vec<_>>>()?;

    for (user, _user_state) in above_triggered {
        let TriggeredOrderOutcome {
            state: updated_state,
            pair_state: updated_pair_state,
        } = process_triggered_order(
            storage,
            querier,
            contract,
            current_time,
            oracle_querier,
            param,
            state,
            pair_id,
            &pair_param,
            &pair_state,
            user,
            TriggerDirection::Above,
            oracle_price,
            events,
        )?;
        *state = updated_state;
        pair_state = updated_pair_state;
    }

    // BELOW orders: trigger when oracle_price <= trigger_price.
    // Keys store inverted trigger_price, so stored <= !oracle_price ≡ real >= oracle_price.
    let below_triggered = USER_STATES
        .idx
        .conditional_orders
        .sub_prefix(pair_id.clone())
        .prefix_range(
            storage,
            Some(PrefixBound::Inclusive((
                TriggerDirection::Below,
                UsdPrice::MIN,
                Uint64::MIN,
            ))),
            Some(PrefixBound::Inclusive((
                TriggerDirection::Below,
                !oracle_price,
                Uint64::MAX,
            ))),
            IterationOrder::Ascending,
        )
        .collect::<StdResult<Vec<_>>>()?;

    for (user, _user_state) in below_triggered {
        let TriggeredOrderOutcome {
            state: updated_state,
            pair_state: updated_pair_state,
        } = process_triggered_order(
            storage,
            querier,
            contract,
            current_time,
            oracle_querier,
            param,
            state,
            pair_id,
            &pair_param,
            &pair_state,
            user,
            TriggerDirection::Below,
            oracle_price,
            events,
        )?;
        *state = updated_state;
        pair_state = updated_pair_state;
    }

    PAIR_STATES.save(storage, pair_id, &pair_state)?;

    Ok(())
}

/// Owned outcome of a `process_triggered_order` call.
///
/// Carries only the post-call copies of `state` and `pair_state` —
/// every other piece of state the function touches is loaded inside
/// the function (`user_state` from `USER_STATES`, `next_order_id`
/// from `NEXT_ORDER_ID`) and written back to storage inline. The
/// caller (`process_conditional_orders_for_pair`) maintains a rolling
/// `state` / `pair_state` across multiple invocations and writes them
/// back at the end of the per-pair loop, so we have to thread the
/// updated copies out via the outcome.
#[derive(Debug)]
pub struct TriggeredOrderOutcome {
    pub state: State,
    pub pair_state: PairState,
}

/// Process a single triggered conditional order: verify position, clamp size,
/// submit a market order to close. Pure w.r.t. `state` and `pair_state` —
/// takes them by `&` and returns the updated copies in the outcome. On the
/// graceful-cancel path (slippage / no liquidity / position closed), the
/// returned `state` and `pair_state` equal the inputs because `compute_submit_order_outcome`
/// is itself pure and never touched them.
fn process_triggered_order(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    contract: Addr,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    state: &State,
    pair_id: &PairId,
    pair_param: &PairParam,
    pair_state: &PairState,
    user: Addr,
    trigger_direction: TriggerDirection,
    oracle_price: UsdPrice,
    events: &mut EventBuilder,
) -> anyhow::Result<TriggeredOrderOutcome> {
    let mut user_state = USER_STATES.may_load(storage, user)?.unwrap_or_default();

    // Extract the conditional order and position size info before mutating.
    let (order, position_size) = match user_state.positions.get(pair_id) {
        Some(pos) => {
            let ord = match trigger_direction {
                TriggerDirection::Above => pos.conditional_order_above.clone(),
                TriggerDirection::Below => pos.conditional_order_below.clone(),
            };
            (ord, Some(pos.size))
        },
        None => (None, None),
    };

    // Clear the conditional order field BEFORE saving so the MultiIndex is updated.
    if let Some(pos) = user_state.positions.get_mut(pair_id) {
        match trigger_direction {
            TriggerDirection::Above => pos.conditional_order_above = None,
            TriggerDirection::Below => pos.conditional_order_below = None,
        }
    }

    // ------------- Pre-trigger check 1. position closed/flipped --------------

    let should_cancel = match (&order, position_size) {
        (Some(ord), Some(pos_size)) => {
            // If size is specified, check for position flip.
            // E.g. order.size is negative (close long) but position is now short.
            match ord.size {
                Some(size) => {
                    (size.is_negative() && pos_size.is_negative())
                        || (size.is_positive() && pos_size.is_positive())
                },
                // size = None means "close entire position" — never flipped.
                None => false,
            }
        },
        _ => true,
    };

    if should_cancel {
        events.push(ConditionalOrderRemoved {
            pair_id: pair_id.clone(),
            user,
            trigger_direction,
            reason: ReasonForOrderRemoval::PositionClosed,
        })?;

        if user_state.is_empty() {
            USER_STATES.remove(storage, user)?;
        } else {
            USER_STATES.save(storage, user, &user_state)?;
        }

        #[cfg(feature = "tracing")]
        {
            tracing::info!(
                %pair_id,
                %user,
                ?trigger_direction,
                "Conditional order cancelled: position closed"
            );
        }

        // Graceful cancel: caller's `state` / `pair_state` are unchanged.
        return Ok(TriggeredOrderOutcome {
            state: state.clone(),
            pair_state: pair_state.clone(),
        });
    }

    // ------------------- Pre-trigger check 2. price banding ------------------

    let order = order.unwrap();

    // If governance has tightened `max_market_slippage` since the order
    // was submitted, the stored `order.max_slippage` may now exceed the
    // cap. Cancel it here instead of submitting to the matching engine.
    if order.max_slippage > pair_param.max_market_slippage {
        events.push(ConditionalOrderRemoved {
            pair_id: pair_id.clone(),
            user,
            trigger_direction,
            reason: ReasonForOrderRemoval::SlippageCapTightened,
        })?;

        if user_state.is_empty() {
            USER_STATES.remove(storage, user)?;
        } else {
            USER_STATES.save(storage, user, &user_state)?;
        }

        #[cfg(feature = "tracing")]
        {
            tracing::info!(
                %pair_id,
                %user,
                ?trigger_direction,
                stored_max_slippage = %order.max_slippage,
                current_cap = %pair_param.max_market_slippage,
                "Conditional order cancelled: slippage cap tightened"
            );
        }

        return Ok(TriggeredOrderOutcome {
            state: state.clone(),
            pair_state: pair_state.clone(),
        });
    }

    // ------------ Triggered: clamp size, send to matching engine -------------

    let position_size = position_size.unwrap();

    // Compute the closing size.
    // If size is None, close the entire position (negate position size).
    // If size is Some, clamp |order.size| to |position.size|.
    let clamped_size = match order.size {
        None => position_size.checked_neg()?,
        Some(size) => {
            let abs_order_size = size.checked_abs()?;
            let abs_pos_size = position_size.checked_abs()?;
            if abs_order_size > abs_pos_size {
                if size.is_negative() {
                    abs_pos_size.checked_neg()?
                } else {
                    abs_pos_size
                }
            } else {
                size
            }
        },
    };

    events.push(ConditionalOrderTriggered {
        pair_id: pair_id.clone(),
        user,
        trigger_price: order.trigger_price,
        trigger_direction,
        oracle_price,
    })?;

    // `compute_submit_order_outcome` is pure: takes `state` / `pair_state` / `user_state`
    // by `&` and returns updated copies in its outcome. On `Err`, the
    // caller's locals are untouched by construction, so the graceful-cancel
    // path can simply discard the outcome and return the input `state` /
    // `pair_state` unchanged. (This is the structural fix for the
    // testnet STP-leak bug — the snapshot/restore workaround that used to
    // live here is no longer necessary.)
    let SubmitOrderOutcome {
        state,
        pair_state,
        taker_state: user_state,
        mut maker_states,
        order_mutations,
        order_to_store: _order_to_store,
        next_order_id,
        next_fill_id,
        index_updates,
        volumes,
        fee_breakdowns,
    } = match compute_submit_order_outcome(
        storage,
        user,
        contract,
        current_time,
        oracle_querier,
        param,
        state,
        pair_id,
        pair_param,
        pair_state,
        &user_state,
        oracle_price,
        clamped_size,
        OrderKind::Market {
            max_slippage: order.max_slippage,
        },
        true, // reduce_only
        None, // tp
        None, // sl
        events,
    ) {
        Err(_) => {
            // Order couldn't fill (slippage exceeded or no liquidity).
            // Cancel it gracefully — don't block other orders.
            events.push(ConditionalOrderRemoved {
                pair_id: pair_id.clone(),
                user,
                trigger_direction,
                reason: ReasonForOrderRemoval::SlippageExceeded,
            })?;

            if user_state.is_empty() {
                USER_STATES.remove(storage, user)?;
            } else {
                USER_STATES.save(storage, user, &user_state)?;
            }

            #[cfg(feature = "tracing")]
            {
                tracing::info!(
                    %pair_id,
                    %user,
                    ?trigger_direction,
                    "Conditional order cancelled: slippage exceeded"
                );
            }

            return Ok(TriggeredOrderOutcome {
                state: state.clone(),
                pair_state: pair_state.clone(),
            });
        },
        Ok(outcome) => outcome,
    };

    // Apply state changes (same pattern as submit_order's section 3).
    flush_volumes(storage, current_time, &volumes)?;

    maker_states.insert(user, user_state);

    let FeeCommissionsOutcome {
        user_states: updated_maker_states,
    } = apply_fee_commissions(
        storage,
        querier,
        contract,
        current_time,
        param,
        &maker_states,
        fee_breakdowns,
        &volumes,
        events,
    )?;
    maker_states = updated_maker_states;

    NEXT_ORDER_ID.save(storage, &next_order_id)?;
    NEXT_FILL_ID.save(storage, &next_fill_id)?;

    for (addr, user_state) in &maker_states {
        USER_STATES.save(storage, *addr, user_state)?;
    }

    apply_position_index_updates(storage, &index_updates)?;

    let is_buy = clamped_size.is_positive();
    let (maker_book, _taker_book) = if is_buy {
        (ASKS, BIDS)
    } else {
        (BIDS, ASKS)
    };

    for (stored_price, maker_order_id, mutation, pre_fill_abs_size) in order_mutations {
        let order_key = (pair_id.clone(), stored_price, maker_order_id);
        let maker_is_bid = !is_buy;
        let real_price = may_invert_price(stored_price, maker_is_bid);

        decrease_liquidity_depths(
            storage,
            pair_id,
            maker_is_bid,
            real_price,
            pre_fill_abs_size,
            &pair_param.bucket_sizes,
        )?;

        match mutation {
            Some(maker_order) => {
                increase_liquidity_depths(
                    storage,
                    pair_id,
                    maker_is_bid,
                    real_price,
                    maker_order.size.checked_abs()?,
                    &pair_param.bucket_sizes,
                )?;

                maker_book.save(storage, order_key, &maker_order)?;
            },
            None => {
                maker_book.remove(storage, order_key)?;
            },
        }
    }

    Ok(TriggeredOrderOutcome { state, pair_state })
}
