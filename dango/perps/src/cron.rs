use {
    crate::{
        ASKS, BIDS, CONDITIONAL_ABOVE, CONDITIONAL_BELOW, NEXT_ORDER_ID, PAIR_IDS, PAIR_PARAMS,
        PAIR_STATES, PARAM, STATE, USER_STATES,
        core::{compute_funding_delta, compute_impact_price, compute_premium},
        flush_volumes,
        liquidity_depth::{decrease_liquidity_depths, increase_liquidity_depths},
        position_index::apply_position_index_updates,
        price::may_invert_price,
        referral::apply_fee_commissions,
        trade::{_cancel_conditional_orders_for_pair, _submit_order},
    },
    dango_oracle::OracleQuerier,
    dango_types::{
        Days, UsdPrice, UsdValue,
        perps::{
            ConditionalOrder, ConditionalOrderId, ConditionalOrderRemoved,
            ConditionalOrderTriggered, LiquidityReleased, OrderKind, PairId, PairParam, PairState,
            Param, ReasonForOrderRemoval, State, UserState,
        },
    },
    grug::{
        Addr, EventBuilder, Order as IterationOrder, PrefixBound, QuerierWrapper, StdResult,
        Storage, Timestamp,
    },
};
#[cfg(feature = "metrics")]
use {
    crate::{NoCachePerpQuerier, core::compute_user_equity},
    std::time::Instant,
};

/// Pop matured unlocks from each user and credit the released USD value back
/// to their trading margin.
pub fn process_unlocks(
    storage: &mut dyn Storage,
    current_time: Timestamp,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    // Load all users whose earliest unlock has matured.
    let users = USER_STATES
        .idx
        .earliest_unlock_end_time
        .prefix_range(
            storage,
            None,
            Some(PrefixBound::Inclusive(current_time)),
            IterationOrder::Ascending,
        )
        .map(|res| {
            let (_timestamp, user, user_state) = res?;
            Ok((user, user_state))
        })
        .collect::<StdResult<Vec<_>>>()?;

    #[cfg(feature = "tracing")]
    let num_users = users.len();

    for (user, user_state) in users {
        process_unlock_for_user(storage, current_time, user, user_state, events)?;
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!(num_users, "Processed matured unlocks");
    }

    Ok(())
}

fn process_unlock_for_user(
    storage: &mut dyn Storage,
    current_time: Timestamp,
    user: Addr,
    mut user_state: UserState,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    let mut amount_usd = UsdValue::ZERO;

    // Loop through unlocks, pop the ones that have matured, sum up USD value
    // of all that have matured.
    while let Some(unlock) = user_state.unlocks.front() {
        if unlock.end_time > current_time {
            break;
        }

        amount_usd.checked_add_assign(unlock.amount_to_release)?;
        user_state.unlocks.pop_front();
    }

    // Credit the released USD value back to the user's trading margin.
    user_state.margin.checked_add_assign(amount_usd)?;

    if amount_usd.is_positive() {
        events.push(LiquidityReleased {
            user,
            amount: amount_usd,
        })?;
    }

    // Save the updated user state to storage.
    if user_state.is_empty() {
        USER_STATES.remove(storage, user)?;
    } else {
        USER_STATES.save(storage, user, &user_state)?;
    }

    Ok(())
}

/// Compute and apply funding deltas for each trading pair using a point-in-time
/// premium snapshot from the order book.
pub fn process_funding(
    storage: &mut dyn Storage,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<()> {
    let param = PARAM.load(storage)?;
    let mut state = STATE.load(storage)?;

    // Only process funding if sufficient time has elapsed since the last funding time.
    let elapsed = current_time - state.last_funding_time;
    if elapsed < param.funding_period {
        return Ok(());
    }

    let interval = Days::from_duration(elapsed)?;
    let pair_ids = PAIR_IDS.load(storage)?;

    for pair_id in pair_ids {
        process_funding_for_pair(storage, oracle_querier, interval, pair_id)?;
    }

    state.last_funding_time = current_time;

    STATE.save(storage, &state)?;

    Ok(())
}

fn process_funding_for_pair(
    storage: &mut dyn Storage,
    oracle_querier: &mut OracleQuerier,
    interval: Days,
    pair_id: PairId,
) -> anyhow::Result<()> {
    let pair_param = PAIR_PARAMS.load(storage, &pair_id)?;
    let mut pair_state = PAIR_STATES.load(storage, &pair_id)?;

    let oracle_price = oracle_querier.query_price_for_perps(&pair_id)?;

    // Walk the bid side: stored in ascending order of inverted price,
    // so ascending iteration gives best-bid-first. Un-invert to get
    // real prices. Bid order sizes are guaranteed positive.
    let bid_iter = BIDS
        .prefix(pair_id.clone())
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let ((stored_price, _), order) = res?;
            let real_price = may_invert_price(stored_price, true);
            Ok((real_price, order.size))
        });

    // Walk the ask side: stored naturally in ascending price order,
    // so ascending iteration gives best-ask-first.
    let ask_iter = ASKS
        .prefix(pair_id.clone())
        .range(storage, None, None, IterationOrder::Ascending)
        .map(|res| {
            let ((stored_price, _), order) = res?;
            Ok((stored_price, order.size.checked_abs()?))
        });

    let impact_bid = compute_impact_price(bid_iter, pair_param.impact_size)?;
    let impact_ask = compute_impact_price(ask_iter, pair_param.impact_size)?;

    let premium = compute_premium(impact_bid, impact_ask, oracle_price)?;

    let funding_delta = compute_funding_delta(
        premium,
        oracle_price,
        pair_param.max_abs_funding_rate,
        interval,
    )?;

    (pair_state.funding_per_unit).checked_add_assign(funding_delta)?;

    PAIR_STATES.save(storage, &pair_id, &pair_state)?;

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            %pair_id,
            %funding_delta,
            "Applied funding delta"
        );
    }

    #[cfg(feature = "metrics")]
    {
        metrics::gauge!(
            crate::metrics::LABEL_FUNDING_RATE,
            "pair_id" => pair_id.to_string()
        )
        .set(pair_state.funding_per_unit.to_f64());
    }

    Ok(())
}

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
    let above_triggered = CONDITIONAL_ABOVE
        .prefix(pair_id.clone())
        .prefix_range(
            storage,
            None,
            Some(PrefixBound::Inclusive(oracle_price)),
            IterationOrder::Ascending,
        )
        .collect::<StdResult<Vec<_>>>()?;

    for ((trigger_price, order_id), order) in above_triggered {
        CONDITIONAL_ABOVE.remove(storage, (pair_id.clone(), trigger_price, order_id))?;

        process_triggered_order(
            storage,
            querier,
            contract,
            current_time,
            oracle_querier,
            param,
            state,
            pair_id,
            &pair_param,
            &mut pair_state,
            order_id,
            order,
            oracle_price,
            events,
        )?;
    }

    // BELOW orders: trigger when oracle_price <= trigger_price.
    // Keys store inverted trigger_price, so stored <= !oracle_price ≡ real >= oracle_price.
    let below_triggered = CONDITIONAL_BELOW
        .prefix(pair_id.clone())
        .prefix_range(
            storage,
            None,
            Some(PrefixBound::Inclusive(!oracle_price)),
            IterationOrder::Ascending,
        )
        .collect::<StdResult<Vec<_>>>()?;

    for ((trigger_price, order_id), order) in below_triggered {
        CONDITIONAL_BELOW.remove(storage, (pair_id.clone(), trigger_price, order_id))?;

        process_triggered_order(
            storage,
            querier,
            contract,
            current_time,
            oracle_querier,
            param,
            state,
            pair_id,
            &pair_param,
            &mut pair_state,
            order_id,
            order,
            oracle_price,
            events,
        )?;
    }

    PAIR_STATES.save(storage, pair_id, &pair_state)?;

    Ok(())
}

/// Process a single triggered conditional order: verify position, clamp size,
/// submit a market order to close.
fn process_triggered_order(
    storage: &mut dyn Storage,
    querier: QuerierWrapper,
    contract: Addr,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    state: &mut State,
    pair_id: &PairId,
    pair_param: &PairParam,
    pair_state: &mut PairState,
    order_id: ConditionalOrderId,
    order: ConditionalOrder,
    oracle_price: UsdPrice,
    events: &mut EventBuilder,
) -> anyhow::Result<()> {
    let mut user_state = USER_STATES
        .may_load(storage, order.user)?
        .unwrap_or_default();

    // Check if the user still has a position in this pair.
    let position = user_state.positions.get(pair_id);

    let should_cancel = match position {
        Some(pos) => {
            // Position flipped: the conditional order's size no longer reduces.
            // E.g. order.size is negative (close long) but position is now short.
            (order.size.is_negative() && pos.size.is_negative())
                || (order.size.is_positive() && pos.size.is_positive())
        },
        None => true,
    };

    if should_cancel {
        events.push(ConditionalOrderRemoved {
            order_id,
            pair_id: pair_id.clone(),
            user: order.user,
            reason: ReasonForOrderRemoval::PositionClosed,
        })?;

        if user_state.is_empty() {
            USER_STATES.remove(storage, order.user)?;
        } else {
            USER_STATES.save(storage, order.user, &user_state)?;
        }

        #[cfg(feature = "tracing")]
        {
            tracing::info!(
                %order_id,
                %pair_id,
                user = %order.user,
                "Conditional order cancelled: position closed"
            );
        }

        return Ok(());
    }

    let position = position.unwrap();

    // Auto-resize: clamp |order.size| to |position.size|.
    let abs_order_size = order.size.checked_abs()?;
    let abs_pos_size = position.size.checked_abs()?;
    let clamped_size = if abs_order_size > abs_pos_size {
        // Preserve the sign of order.size but clamp the magnitude.
        if order.size.is_negative() {
            abs_pos_size.checked_neg()?
        } else {
            abs_pos_size
        }
    } else {
        order.size
    };

    events.push(ConditionalOrderTriggered {
        order_id,
        pair_id: pair_id.clone(),
        user: order.user,
        trigger_price: order.trigger_price,
        oracle_price,
    })?;

    // Execute as a market order via `_submit_order`.
    let result = _submit_order(
        storage,
        order.user,
        contract,
        current_time,
        oracle_querier,
        param,
        state,
        pair_id,
        pair_param,
        pair_state,
        &mut user_state,
        oracle_price,
        clamped_size,
        OrderKind::Market {
            max_slippage: order.max_slippage,
        },
        true, // reduce_only
        events,
    );

    let (
        mut maker_states,
        order_mutations,
        _order_to_store,
        next_order_id,
        index_updates,
        volumes,
        fee_breakdowns,
    ) = match result {
        Err(_) => {
            // Order couldn't fill (slippage exceeded or no liquidity).
            // Cancel it gracefully — don't block other orders.
            events.push(ConditionalOrderRemoved {
                order_id,
                pair_id: pair_id.clone(),
                user: order.user,
                reason: ReasonForOrderRemoval::SlippageExceeded,
            })?;

            if user_state.is_empty() {
                USER_STATES.remove(storage, order.user)?;
            } else {
                USER_STATES.save(storage, order.user, &user_state)?;
            }

            #[cfg(feature = "tracing")]
            {
                tracing::info!(
                    %order_id,
                    %pair_id,
                    user = %order.user,
                    "Conditional order cancelled: slippage exceeded"
                );
            }

            return Ok(());
        },
        Ok(tuple) => tuple,
    };

    // Apply state changes (same pattern as submit_order's section 3).
    flush_volumes(storage, current_time, &volumes)?;

    maker_states.insert(order.user, user_state);

    // Cancel sibling conditional orders for users whose positions were fully
    // closed or flipped by this triggered order.
    for update in &index_updates {
        let should_cancel = match (&update.old_entry, &update.new_entry) {
            (Some(_), None) => true,
            (Some((_, was_long)), Some((_, is_long))) if was_long != is_long => true,
            _ => false,
        };

        if should_cancel {
            _cancel_conditional_orders_for_pair(
                storage,
                update.user,
                &update.pair_id,
                ReasonForOrderRemoval::PositionClosed,
                events,
            )?;
        }
    }

    apply_fee_commissions(
        storage,
        querier,
        contract,
        current_time,
        param,
        &mut maker_states,
        fee_breakdowns,
        &volumes,
        events,
    )?;

    NEXT_ORDER_ID.save(storage, &next_order_id)?;

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

    Ok(())
}

/// Emit metrics gauges and histograms captured during cron execution.
///
/// Loads vault state to report equity, margin, positions, insurance fund,
/// and treasury gauges, then records the cron duration histogram.
#[cfg(feature = "metrics")]
pub fn emit_cron_metrics(
    storage: &dyn Storage,
    contract: Addr,
    oracle_querier: &mut OracleQuerier,
    start: Instant,
) -> anyhow::Result<()> {
    let state = STATE.load(storage)?;
    let vault_state = USER_STATES.may_load(storage, contract)?.unwrap_or_default();
    let perp_querier = NoCachePerpQuerier::new_local(storage);

    if let Ok(vault_equity) = compute_user_equity(oracle_querier, &perp_querier, &vault_state) {
        metrics::gauge!(crate::metrics::LABEL_VAULT_EQUITY).set(vault_equity.to_f64());
    }

    for (pair_id, position) in &vault_state.positions {
        metrics::gauge!(
            crate::metrics::LABEL_VAULT_POSITION,
            "pair_id" => pair_id.to_string()
        )
        .set(position.size.to_f64());
    }

    metrics::gauge!(crate::metrics::LABEL_VAULT_MARGIN).set(vault_state.margin.to_f64());

    metrics::gauge!(crate::metrics::LABEL_INSURANCE_FUND).set(state.insurance_fund.to_f64());

    metrics::gauge!(crate::metrics::LABEL_TREASURY).set(state.treasury.to_f64());

    metrics::histogram!(crate::metrics::LABEL_DURATION_CRON).record(start.elapsed().as_secs_f64());

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            FundingPerUnit, FundingRate, Quantity, UsdPrice,
            oracle::PrecisionedPrice,
            perps::{LimitOrder, PairParam, PairState, Param, State, Unlock},
        },
        grug::{Duration, MockStorage, Udec128, Uint64, hash_map},
        std::collections::{BTreeSet, VecDeque},
    };

    const USER_A: Addr = Addr::mock(1);
    const USER_B: Addr = Addr::mock(2);
    const MAKER: Addr = Addr::mock(3);

    fn btc_pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn eth_pair_id() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    /// Build unlocks from `(usd_amount, end_time_seconds)` pairs.
    ///
    /// Mutates: nothing.
    /// Returns: a `VecDeque<Unlock>` for use in `UserState`.
    fn unlocks_from(entries: &[(i128, u128)]) -> VecDeque<Unlock> {
        entries
            .iter()
            .map(|&(amount, secs)| Unlock {
                amount_to_release: UsdValue::new_int(amount),
                end_time: Timestamp::from_seconds(secs),
            })
            .collect()
    }

    /// Place a resting bid order into `BIDS` storage.
    ///
    /// Mutates: writes to `storage`.
    /// Returns: nothing.
    fn place_bid_order(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        price: i128,
        size: i128,
        order_id: u64,
    ) {
        let inverted_price = !UsdPrice::new_int(price);
        let key = (pair_id.clone(), inverted_price, Uint64::new(order_id));
        let order = LimitOrder {
            user: MAKER,
            size: Quantity::new_int(size.abs()),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
            created_at: Timestamp::ZERO,
        };
        BIDS.save(storage, key, &order).unwrap();
    }

    /// Place a resting ask order into `ASKS` storage.
    ///
    /// Mutates: writes to `storage`.
    /// Returns: nothing.
    fn place_ask_order(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        price: i128,
        size: i128,
        order_id: u64,
    ) {
        let key = (
            pair_id.clone(),
            UsdPrice::new_int(price),
            Uint64::new(order_id),
        );
        let order = LimitOrder {
            user: MAKER,
            size: Quantity::new_int(-size.abs()),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
            created_at: Timestamp::ZERO,
        };
        ASKS.save(storage, key, &order).unwrap();
    }

    /// Save the common funding-related storage items for a single pair.
    ///
    /// Mutates: writes `PARAM`, `STATE`, `PAIR_IDS`, `PAIR_PARAMS`, `PAIR_STATES`
    /// to `storage`.
    /// Returns: nothing.
    fn init_funding_storage(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        pair_param: &PairParam,
        pair_state: &PairState,
        funding_period_secs: u128,
        last_funding_time_secs: u128,
    ) {
        PARAM
            .save(storage, &Param {
                funding_period: Duration::from_seconds(funding_period_secs),
                ..Default::default()
            })
            .unwrap();
        STATE
            .save(storage, &State {
                last_funding_time: Timestamp::from_seconds(last_funding_time_secs),
                ..Default::default()
            })
            .unwrap();
        PAIR_IDS
            .save(storage, &BTreeSet::from([pair_id.clone()]))
            .unwrap();
        PAIR_PARAMS.save(storage, pair_id, pair_param).unwrap();
        PAIR_STATES.save(storage, pair_id, pair_state).unwrap();
    }

    fn default_funding_pair_param() -> PairParam {
        PairParam {
            impact_size: UsdValue::new_int(10_000),
            max_abs_funding_rate: FundingRate::new_raw(50_000), // 0.05/day
            ..Default::default()
        }
    }

    // ==================== process_unlocks tests ====================

    #[test]
    fn no_matured_unlocks_unchanged() {
        let mut storage = MockStorage::new();

        let user_state = UserState {
            unlocks: unlocks_from(&[(1000, 200), (2000, 300)]),
            ..Default::default()
        };
        USER_STATES.save(&mut storage, USER_A, &user_state).unwrap();

        process_unlocks(
            &mut storage,
            Timestamp::from_seconds(100),
            &mut EventBuilder::new(),
        )
        .unwrap();

        let loaded = USER_STATES.load(&storage, USER_A).unwrap();
        assert_eq!(loaded.unlocks.len(), 2);
        assert_eq!(loaded.margin, UsdValue::ZERO);
    }

    #[test]
    fn single_user_single_matured_unlock() {
        let mut storage = MockStorage::new();

        let user_state = UserState {
            unlocks: unlocks_from(&[(1000, 100)]),
            ..Default::default()
        };
        USER_STATES.save(&mut storage, USER_A, &user_state).unwrap();

        // At t=100 the unlock matures (end_time > current_time is false).
        process_unlocks(
            &mut storage,
            Timestamp::from_seconds(100),
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Margin credited, unlocks cleared. User state persists because margin > 0.
        let loaded = USER_STATES.load(&storage, USER_A).unwrap();
        assert_eq!(loaded.margin, UsdValue::new_int(1000));
        assert!(loaded.unlocks.is_empty());
    }

    #[test]
    fn single_user_partial_maturation() {
        let mut storage = MockStorage::new();

        let user_state = UserState {
            unlocks: unlocks_from(&[(1000, 100), (2000, 200), (3000, 300)]),
            ..Default::default()
        };
        USER_STATES.save(&mut storage, USER_A, &user_state).unwrap();

        // At t=200 the first two unlocks mature ($1000 + $2000 = $3000).
        process_unlocks(
            &mut storage,
            Timestamp::from_seconds(200),
            &mut EventBuilder::new(),
        )
        .unwrap();

        let loaded = USER_STATES.load(&storage, USER_A).unwrap();
        assert_eq!(loaded.margin, UsdValue::new_int(3000));
        assert_eq!(loaded.unlocks.len(), 1);
        assert_eq!(loaded.unlocks[0].amount_to_release, UsdValue::new_int(3000));
    }

    #[test]
    fn multiple_users_margin_credited() {
        let mut storage = MockStorage::new();

        USER_STATES
            .save(&mut storage, USER_A, &UserState {
                unlocks: unlocks_from(&[(500, 50)]),
                ..Default::default()
            })
            .unwrap();
        USER_STATES
            .save(&mut storage, USER_B, &UserState {
                unlocks: unlocks_from(&[(700, 60)]),
                ..Default::default()
            })
            .unwrap();

        process_unlocks(
            &mut storage,
            Timestamp::from_seconds(100),
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Both users get margin credited.
        let loaded_a = USER_STATES.load(&storage, USER_A).unwrap();
        assert_eq!(loaded_a.margin, UsdValue::new_int(500));
        assert!(loaded_a.unlocks.is_empty());

        let loaded_b = USER_STATES.load(&storage, USER_B).unwrap();
        assert_eq!(loaded_b.margin, UsdValue::new_int(700));
        assert!(loaded_b.unlocks.is_empty());
    }

    #[test]
    fn user_with_margin_preserved_after_unlock() {
        let mut storage = MockStorage::new();

        // User has unlocks AND nonzero margin.
        let user_state = UserState {
            margin: UsdValue::new_int(500),
            unlocks: unlocks_from(&[(1000, 100)]),
            ..Default::default()
        };
        USER_STATES.save(&mut storage, USER_A, &user_state).unwrap();

        process_unlocks(
            &mut storage,
            Timestamp::from_seconds(200),
            &mut EventBuilder::new(),
        )
        .unwrap();

        // User state persists, margin = original $500 + released $1000 = $1500.
        let loaded = USER_STATES.load(&storage, USER_A).unwrap();
        assert_eq!(loaded.margin, UsdValue::new_int(1500));
        assert!(loaded.unlocks.is_empty());
    }

    #[test]
    fn no_users_no_error() {
        let mut storage = MockStorage::new();

        process_unlocks(
            &mut storage,
            Timestamp::from_seconds(100),
            &mut EventBuilder::new(),
        )
        .unwrap();
    }

    // ==================== process_funding tests ====================

    #[test]
    fn funding_skipped_when_period_not_elapsed() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        init_funding_storage(
            &mut storage,
            &pair_id,
            &default_funding_pair_param(),
            &PairState::default(),
            3600,
            0,
        );

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        // Only 1800s elapsed, period is 3600s → funding skipped.
        process_funding(&mut storage, Timestamp::from_seconds(1800), &mut oracle).unwrap();

        let state = STATE.load(&storage).unwrap();
        assert_eq!(state.last_funding_time, Timestamp::from_seconds(0));

        let pair_state = PAIR_STATES.load(&storage, &pair_id).unwrap();
        assert_eq!(pair_state.funding_per_unit, FundingPerUnit::ZERO);
    }

    #[test]
    fn funding_applied_when_period_elapsed() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        init_funding_storage(
            &mut storage,
            &pair_id,
            &default_funding_pair_param(),
            &PairState::default(),
            3600,
            0,
        );

        // Bid at $51,000 above oracle $50,000 → positive premium.
        place_bid_order(&mut storage, &pair_id, 51_000, 1, 1);

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        process_funding(&mut storage, Timestamp::from_seconds(3600), &mut oracle).unwrap();

        let state = STATE.load(&storage).unwrap();
        assert_eq!(state.last_funding_time, Timestamp::from_seconds(3600));

        let pair_state = PAIR_STATES.load(&storage, &pair_id).unwrap();
        assert_ne!(pair_state.funding_per_unit, FundingPerUnit::ZERO);
    }

    #[test]
    fn funding_with_empty_order_book() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();

        init_funding_storage(
            &mut storage,
            &pair_id,
            &default_funding_pair_param(),
            &PairState::default(),
            3600,
            0,
        );

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        process_funding(&mut storage, Timestamp::from_seconds(3600), &mut oracle).unwrap();

        let state = STATE.load(&storage).unwrap();
        assert_eq!(state.last_funding_time, Timestamp::from_seconds(3600));

        // Empty book → premium = 0 → delta = 0.
        let pair_state = PAIR_STATES.load(&storage, &pair_id).unwrap();
        assert_eq!(pair_state.funding_per_unit, FundingPerUnit::ZERO);
    }

    #[test]
    fn funding_multiple_pairs() {
        let mut storage = MockStorage::new();
        let btc = btc_pair_id();
        let eth = eth_pair_id();
        let pair_param = default_funding_pair_param();

        PARAM
            .save(&mut storage, &Param {
                funding_period: Duration::from_seconds(3600),
                ..Default::default()
            })
            .unwrap();
        STATE
            .save(&mut storage, &State {
                last_funding_time: Timestamp::from_seconds(0),
                ..Default::default()
            })
            .unwrap();
        PAIR_IDS
            .save(&mut storage, &BTreeSet::from([btc.clone(), eth.clone()]))
            .unwrap();
        PAIR_PARAMS.save(&mut storage, &btc, &pair_param).unwrap();
        PAIR_PARAMS.save(&mut storage, &eth, &pair_param).unwrap();
        PAIR_STATES
            .save(&mut storage, &btc, &PairState::default())
            .unwrap();
        PAIR_STATES
            .save(&mut storage, &eth, &PairState::default())
            .unwrap();

        // BTC: bid above oracle → positive premium.
        place_bid_order(&mut storage, &btc, 51_000, 1, 1);
        // ETH: ask below oracle → negative premium.
        place_ask_order(&mut storage, &eth, 2_900, 10, 2);

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            btc.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
            eth.clone() => PrecisionedPrice::new(
                Udec128::new_percent(300_000), // $3,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        process_funding(&mut storage, Timestamp::from_seconds(3600), &mut oracle).unwrap();

        let state = STATE.load(&storage).unwrap();
        assert_eq!(state.last_funding_time, Timestamp::from_seconds(3600));

        let btc_state = PAIR_STATES.load(&storage, &btc).unwrap();
        assert_ne!(btc_state.funding_per_unit, FundingPerUnit::ZERO);

        let eth_state = PAIR_STATES.load(&storage, &eth).unwrap();
        assert_ne!(eth_state.funding_per_unit, FundingPerUnit::ZERO);
    }

    #[test]
    fn funding_updates_accumulator_not_replaces() {
        let mut storage = MockStorage::new();
        let pair_id = btc_pair_id();
        let initial_funding = FundingPerUnit::new_int(100);

        init_funding_storage(
            &mut storage,
            &pair_id,
            &default_funding_pair_param(),
            &PairState {
                funding_per_unit: initial_funding,
                ..Default::default()
            },
            3600,
            0,
        );

        // Bid above oracle → positive delta added to existing accumulator.
        place_bid_order(&mut storage, &pair_id, 51_000, 1, 1);

        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id.clone() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        process_funding(&mut storage, Timestamp::from_seconds(3600), &mut oracle).unwrap();

        let pair_state = PAIR_STATES.load(&storage, &pair_id).unwrap();
        // Accumulator = initial (100) + positive delta, so strictly greater.
        assert!(pair_state.funding_per_unit > initial_funding);
    }
}
