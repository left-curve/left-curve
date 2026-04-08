use {
    crate::{
        VOLUME_LOOKBACK,
        core::{
            check_margin, check_minimum_order_size, check_oi_constraint, compute_available_margin,
            compute_notional, compute_required_margin, compute_target_price, compute_trading_fee,
            decompose_fill, execute_fill, is_price_constraint_violated,
        },
        liquidity_depth::{decrease_liquidity_depths, increase_liquidity_depths},
        oracle,
        position_index::{
            PositionIndexUpdate, apply_position_index_updates, compute_position_diff,
        },
        price::may_invert_price,
        querier::NoCachePerpQuerier,
        query::query_volume,
        referral::{FeeCommissionsOutcome, apply_fee_commissions},
        state::{ASKS, BIDS, NEXT_ORDER_ID, PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES},
        volume::flush_volumes,
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        perps::{
            ChildOrder, ConditionalOrder, ConditionalOrderPlaced, LimitOrder, OrderFilled, OrderId,
            OrderKind, OrderPersisted, OrderRemoved, PairId, PairParam, PairState, Param,
            ReasonForOrderRemoval, State, TriggerDirection, UserState,
        },
    },
    grug::{
        Addr, EventBuilder, MutableCtx, Number, NumberConst, Order as IterationOrder, Response,
        Storage, Timestamp,
    },
    std::collections::{BTreeMap, btree_map::Entry},
};

pub fn submit_order(
    ctx: MutableCtx,
    pair_id: PairId,
    size: Quantity,
    kind: OrderKind,
    reduce_only: bool,
    tp: Option<ChildOrder>,
    sl: Option<ChildOrder>,
) -> anyhow::Result<Response> {
    #[cfg(feature = "metrics")]
    let start = std::time::Instant::now();

    // ---------------------------- 1. Preparation -----------------------------

    let param = PARAM.load(ctx.storage)?;
    let state = STATE.load(ctx.storage)?;

    let pair_param = PAIR_PARAMS.load(ctx.storage, &pair_id)?;
    let pair_state = PAIR_STATES.load(ctx.storage, &pair_id)?;

    let taker_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let mut oracle_querier = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier);

    let oracle_price = oracle_querier.query_price_for_perps(&pair_id)?;

    let mut events = EventBuilder::new();

    // --------------------------- 2. Business logic ---------------------------

    let SubmitOrderOutcome {
        state,
        pair_state,
        taker_state,
        mut maker_states,
        order_mutations,
        order_to_store,
        next_order_id,
        index_updates,
        volumes,
        fee_breakdowns,
    } = _submit_order(
        ctx.storage,
        ctx.sender,
        ctx.contract,
        ctx.block.timestamp,
        &mut oracle_querier,
        &param,
        &state,
        &pair_id,
        &pair_param,
        &pair_state,
        &taker_state,
        oracle_price,
        size,
        kind,
        reduce_only,
        tp,
        sl,
        &mut events,
    )?;

    // ------------------------ 3. Apply state changes -------------------------

    flush_volumes(ctx.storage, ctx.block.timestamp, &volumes)?;

    maker_states.insert(ctx.sender, taker_state);

    let FeeCommissionsOutcome {
        user_states: updated_maker_states,
    } = apply_fee_commissions(
        ctx.storage,
        ctx.querier,
        ctx.contract,
        ctx.block.timestamp,
        &param,
        &maker_states,
        fee_breakdowns,
        &volumes,
        &mut events,
    )?;

    maker_states = updated_maker_states;

    NEXT_ORDER_ID.save(ctx.storage, &next_order_id)?;

    STATE.save(ctx.storage, &state)?;

    PAIR_STATES.save(ctx.storage, &pair_id, &pair_state)?;

    for (addr, user_state) in &maker_states {
        USER_STATES.save(ctx.storage, *addr, user_state)?;
    }

    apply_position_index_updates(ctx.storage, &index_updates)?;

    let (taker_book, maker_book) = if size.is_positive() {
        (BIDS, ASKS)
    } else {
        (ASKS, BIDS)
    };

    for (stored_price, order_id, mutation, pre_fill_abs_size) in order_mutations {
        let order_key = (pair_id.clone(), stored_price, order_id);

        // The maker is on the opposite side of the taker.
        let maker_is_bid = !size.is_positive();
        let real_price = may_invert_price(stored_price, maker_is_bid);

        // Completely remove the old order's liquidity depth contribution.
        // If the order still has some size remaining, we re-add it.
        // Why don't we simply subtract the delta? To avoid a situation known as
        // notional drift. See `../liquidity_depth.rs`, test function `partial_fill_no_residual_depth`
        // for detail.
        decrease_liquidity_depths(
            ctx.storage,
            &pair_id,
            maker_is_bid,
            real_price,
            pre_fill_abs_size,
            &pair_param.bucket_sizes,
        )?;

        match mutation {
            Some(order) => {
                increase_liquidity_depths(
                    ctx.storage,
                    &pair_id,
                    maker_is_bid,
                    real_price,
                    order.size.checked_abs()?,
                    &pair_param.bucket_sizes,
                )?;

                maker_book.save(ctx.storage, order_key, &order)?;
            },
            None => {
                maker_book.remove(ctx.storage, order_key)?;
            },
        }
    }

    if let Some((stored_price, order_id, order)) = order_to_store {
        let is_bid = size.is_positive();
        let limit_price = may_invert_price(stored_price, is_bid);

        increase_liquidity_depths(
            ctx.storage,
            &pair_id,
            is_bid,
            limit_price,
            order.size.checked_abs()?,
            &pair_param.bucket_sizes,
        )?;

        taker_book.save(
            ctx.storage,
            (pair_id.clone(), stored_price, order_id),
            &order,
        )?;

        events.push(OrderPersisted {
            order_id,
            pair_id: pair_id.clone(),
            user: ctx.sender,
            limit_price,
            size: order.size,
        })?;
    }

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            user = %ctx.sender,
            %pair_id,
            %size,
            "Order submitted"
        );
    }

    #[cfg(feature = "metrics")]
    {
        let pair_label = pair_id.to_string();

        metrics::counter!(
            crate::metrics::LABEL_ORDERS_SUBMITTED,
            "pair_id" => pair_label.clone()
        )
        .increment(1);

        metrics::gauge!(
            crate::metrics::LABEL_OPEN_INTEREST_LONG,
            "pair_id" => pair_label.clone()
        )
        .set(pair_state.long_oi.to_f64());

        metrics::gauge!(
            crate::metrics::LABEL_OPEN_INTEREST_SHORT,
            "pair_id" => pair_label.clone()
        )
        .set(pair_state.short_oi.to_f64());

        metrics::histogram!(
            crate::metrics::LABEL_DURATION_SUBMIT_ORDER,
            "pair_id" => pair_label
        )
        .record(start.elapsed().as_secs_f64());
    }

    // No token transfers — all PnL/fees settled via user_state.margin.
    Ok(Response::new().add_events(events)?)
}

/// Owned outcome of a `_submit_order` call. Every piece of
/// caller-persistable state that the call may have updated is returned in
/// this struct, so that a failed call can discard everything at once
/// simply by dropping the `Err` variant — no partial mutations can leak
/// into the caller's `&mut` parameters. See
/// [`dango/perps/purity.md`](../../purity.md) for the full rationale.
#[derive(Debug)]
pub struct SubmitOrderOutcome {
    pub state: State,
    pub pair_state: PairState,
    pub taker_state: UserState,
    pub maker_states: BTreeMap<Addr, UserState>,
    pub order_mutations: Vec<(UsdPrice, OrderId, Option<LimitOrder>, Quantity)>,
    pub order_to_store: Option<(UsdPrice, OrderId, LimitOrder)>,
    pub next_order_id: OrderId,
    pub index_updates: Vec<PositionIndexUpdate>,
    pub volumes: BTreeMap<Addr, UsdValue>,
    pub fee_breakdowns: BTreeMap<Addr, FeeBreakdown>,
}

/// Pure order submission: reads from storage but does not write.
/// Takes the dense state structs (`State`, `PairState`, `UserState`) by
/// shared reference, clones them at entry, and returns the updated
/// copies in [`SubmitOrderOutcome`] alongside the deferred order-book
/// mutations and PnL/fee accumulators. A failed call leaves the
/// caller's inputs untouched — see `dango/perps/purity.md`.
///
/// `apply_fee_commissions` is still called by the entry point on the
/// returned `maker_states`; moving it inside is left as a follow-up
/// since that requires lifting `&mut Storage` and `QuerierWrapper`
/// into the signature and re-plumbing every test call site.
pub(crate) fn _submit_order(
    storage: &dyn Storage,
    taker: Addr,
    contract: Addr,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    state: &State,
    pair_id: &PairId,
    pair_param: &PairParam,
    pair_state: &PairState,
    taker_state: &UserState,
    oracle_price: UsdPrice,
    size: Quantity,
    kind: OrderKind,
    reduce_only: bool,
    tp: Option<ChildOrder>,
    sl: Option<ChildOrder>,
    events: &mut EventBuilder,
) -> anyhow::Result<SubmitOrderOutcome> {
    // Clone at entry and mutate locals freely. `events` is the one
    // deliberate `&mut` on caller state per the purity rule exception.
    let mut state = state.clone();
    let mut pair_state = pair_state.clone();
    let mut taker_state = taker_state.clone();

    // -------------- Step 1. Check minimum order size -------------------------

    if !reduce_only {
        check_minimum_order_size(size, oracle_price, pair_param)?;
    }

    // ----------------------- Step 2. Decompose order -------------------------

    let (closing_size, mut opening_size) = {
        let current_position = taker_state
            .positions
            .get(pair_id)
            .map(|p| p.size)
            .unwrap_or_default();
        decompose_fill(size, current_position)
    };

    if reduce_only {
        opening_size = Quantity::ZERO;
    }

    let fillable_size = closing_size.checked_add(opening_size)?;

    ensure!(fillable_size.is_non_zero(), "fillable size is zero");

    // -------------- Step 3. Check OI constraint for opening ------------------

    check_oi_constraint(opening_size, &pair_state, pair_param)?;

    // --------------- Step 3½. Allocate a unique order ID ---------------------

    let taker_order_id = NEXT_ORDER_ID.load(storage)?;
    let mut next_order_id = taker_order_id + OrderId::ONE;

    // ---------------------- Step 4. Post-only fast path ----------------------

    if let Some(limit_price) = kind.post_only_price() {
        let StoreLimitOrderOutcome {
            user_state: updated_taker_state,
            stored_price,
            order_id,
            order,
        } = store_post_only_limit_order(
            storage,
            taker,
            current_time,
            oracle_querier,
            param,
            pair_id,
            pair_param,
            &taker_state,
            fillable_size,
            limit_price,
            reduce_only,
            taker_order_id,
            tp,
            sl,
        )?;

        taker_state = updated_taker_state;

        return Ok(SubmitOrderOutcome {
            state,
            pair_state,
            taker_state,
            maker_states: BTreeMap::new(),
            order_mutations: Vec::new(),
            order_to_store: Some((stored_price, order_id, order)),
            next_order_id: taker_order_id + OrderId::ONE,
            index_updates: Vec::new(),
            volumes: BTreeMap::new(),
            fee_breakdowns: BTreeMap::new(),
        });
    }

    // ----------------- Step 5: Pre-match taker margin check ------------------
    //
    // Reduce-only orders only reduce exposure, so they skip the check.

    if !reduce_only {
        let perp_querier = NoCachePerpQuerier::new_local(storage);

        let taker_fee_rate = {
            let volume_since = Some(current_time.saturating_sub(VOLUME_LOOKBACK));
            let taker_volume = query_volume(storage, taker, volume_since)?;
            param.taker_fee_rates.resolve(taker_volume)
        };

        check_margin(
            oracle_querier,
            pair_id,
            &perp_querier,
            &taker_state,
            taker_fee_rate,
            oracle_price,
            size,
        )?;
    }

    // --------------------- Step 6. Compute target price ----------------------

    let taker_is_bid = size.is_positive();
    let target_price = compute_target_price(kind, oracle_price, taker_is_bid)?;

    // ---------------------- Step 7. Match against book -----------------------

    let MatchOrderOutcome {
        pair_state: updated_pair_state,
        taker_state: updated_taker_state,
        mut maker_states,
        unfilled,
        pnls,
        fees,
        volumes,
        order_mutations,
        index_updates,
        next_order_id: updated_next_order_id,
    } = match_order(
        storage,
        taker,
        contract,
        current_time,
        param,
        pair_id,
        &pair_state,
        &taker_state,
        taker_is_bid,
        taker_order_id,
        &BTreeMap::new(),
        target_price,
        fillable_size,
        next_order_id,
        events,
    )?;

    pair_state = updated_pair_state;
    taker_state = updated_taker_state;
    next_order_id = updated_next_order_id;

    // ------------------- Step 8. Handle unfilled remainder -------------------

    let order_to_store = if unfilled.is_non_zero() {
        match kind {
            OrderKind::Limit { limit_price, .. } => {
                let StoreLimitOrderOutcome {
                    user_state: updated_taker_state,
                    stored_price,
                    order_id,
                    order,
                } = store_limit_order(
                    storage,
                    taker,
                    current_time,
                    oracle_querier,
                    param,
                    pair_param,
                    &taker_state,
                    unfilled,
                    limit_price,
                    reduce_only,
                    taker_order_id,
                    tp.clone(),
                    sl.clone(),
                )?;

                taker_state = updated_taker_state;

                Some((stored_price, order_id, order))
            },
            OrderKind::Market { .. } => {
                ensure!(
                    unfilled < fillable_size,
                    "no liquidity at acceptable price! target_price: {target_price}"
                );

                None
            },
        }
    } else {
        None
    };

    // ---------- Step 8½. Apply taker's child orders after fills --------------

    // If the taker's order has TP/SL setting attached, apply them if:
    // - the taker order was at least partially filled;
    // - the taker now has a position of non-zero size;
    // - the position is of the same direction as the order.

    let had_fills = unfilled != fillable_size;

    if had_fills
        && (tp.is_some() || sl.is_some())
        && let Some(position) = taker_state.positions.get_mut(pair_id)
        && position.size.is_positive() == size.is_positive()
    {
        let (above, below) = map_child_orders(position.size, &tp, &sl, &mut next_order_id);

        position.conditional_order_above = above;
        position.conditional_order_below = below;

        emit_child_order_events(events, pair_id, taker, &tp, &sl, position.size)?;
    }

    // Ensure the vault's UserState is in maker_states for fee settlement.
    maker_states.entry(contract).or_insert_with(|| {
        USER_STATES
            .may_load(storage, contract)
            .unwrap()
            .unwrap_or_default()
    });

    let fee_breakdowns = settle_pnls(
        contract,
        param,
        &mut state,
        taker,
        &mut taker_state,
        &mut maker_states,
        pnls,
        fees,
    )?;

    Ok(SubmitOrderOutcome {
        state,
        pair_state,
        taker_state,
        maker_states,
        order_mutations,
        order_to_store,
        next_order_id,
        index_updates,
        volumes,
        fee_breakdowns,
    })
}

/// Owned outcome of a `match_order` call. Carries post-match copies of
/// `pair_state`, `taker_state`, and `maker_states`, plus the per-fill
/// accumulators (`pnls`, `fees`, `volumes`) that the caller feeds into
/// `settle_pnls` / `apply_fee_commissions` or merges into its own running
/// totals (`execute_close_schedule`).
#[derive(Debug)]
pub struct MatchOrderOutcome {
    pub pair_state: PairState,
    pub taker_state: UserState,
    pub maker_states: BTreeMap<Addr, UserState>,
    pub unfilled: Quantity,
    pub pnls: BTreeMap<Addr, UsdValue>,
    pub fees: BTreeMap<Addr, UsdValue>,
    pub volumes: BTreeMap<Addr, UsdValue>,
    pub order_mutations: Vec<(UsdPrice, OrderId, Option<LimitOrder>, Quantity)>,
    pub index_updates: Vec<PositionIndexUpdate>,
    pub next_order_id: OrderId,
}

/// Iterate the opposite-side order book in price-time priority and match
/// the taker against resting orders, accumulating fills, self-trade
/// cancellations, and the associated PnL / fee / volume / position-index
/// deltas. Pure w.r.t. the caller's dense state: takes `&PairState`,
/// `&UserState`, `&BTreeMap<Addr, UserState>`, and an owned
/// `next_order_id`; clones each at entry and returns the updated copies
/// in [`MatchOrderOutcome`]. A failed call drops the locals and leaves
/// the caller's inputs untouched — see `dango/perps/purity.md`.
///
/// Self-trade prevention (EXPIRE_MAKER): if a resting order belongs to
/// the taker, the order is cancelled and the taker continues matching
/// deeper in the book. This is consistent with Binance's EXPIRE_MAKER
/// mode: <https://developers.binance.com/docs/binance-spot-api-docs/faqs/stp_faq>
pub fn match_order(
    storage: &dyn Storage,
    taker: Addr,
    contract: Addr,
    current_time: Timestamp,
    param: &Param,
    pair_id: &PairId,
    pair_state: &PairState,
    taker_state: &UserState,
    taker_is_bid: bool,
    taker_order_id: OrderId,
    maker_states: &BTreeMap<Addr, UserState>,
    target_price: UsdPrice,
    mut remaining_size: Quantity,
    mut next_order_id: OrderId,
    events: &mut EventBuilder,
) -> anyhow::Result<MatchOrderOutcome> {
    // Clone at entry and mutate locals freely. `events` is the one
    // deliberate `&mut` on caller state per the purity rule exception.
    let mut pair_state = pair_state.clone();
    let mut taker_state = taker_state.clone();
    let mut maker_states = maker_states.clone();

    let mut pnls = BTreeMap::new();
    let mut fees = BTreeMap::new();
    let mut volumes = BTreeMap::new();
    let mut order_mutations = Vec::new();
    let mut index_updates = Vec::new();

    // Resolve taker's fee rate based on recent volume.
    let volume_since = Some(current_time.saturating_sub(VOLUME_LOOKBACK));
    let taker_fee_rate = {
        let taker_volume = query_volume(storage, taker, volume_since)?;
        param.taker_fee_rates.resolve(taker_volume)
    };

    // Create iterator over the maker side of the order book.
    // The iteration follows price-time priority.
    let maker_book = if taker_is_bid {
        ASKS
    } else {
        BIDS
    };

    let maker_orders =
        maker_book
            .prefix(pair_id.clone())
            .range(storage, None, None, IterationOrder::Ascending);

    for record in maker_orders {
        let ((stored_price, maker_order_id), mut maker_order) = record?;

        // If the maker is bid (i.e. taker is ask), we need to "un-invert" the price.
        let resting_price = may_invert_price(stored_price, !taker_is_bid);

        // ----------------------- Termination condition -----------------------

        if remaining_size.is_zero() {
            break;
        }

        if is_price_constraint_violated(resting_price, target_price, taker_is_bid) {
            break;
        }

        // ----------------------- Self-trade prevention -----------------------

        // If we come across a maker order that was placed by the taker himself,
        // cancel the maker order and move on.
        // This is consistent with industry standard practice. Specifically, it
        // corresponds to Binance's EXPIRE_MAKER mode:
        // https://developers.binance.com/docs/binance-spot-api-docs/faqs/stp_faq
        if maker_order.user == taker {
            let pre_fill_abs_size = maker_order.size.checked_abs()?;

            taker_state.open_order_count -= 1;
            (taker_state.reserved_margin).checked_sub_assign(maker_order.reserved_margin)?;

            order_mutations.push((stored_price, maker_order_id, None, pre_fill_abs_size));

            events.push(OrderRemoved {
                order_id: maker_order_id,
                pair_id: pair_id.clone(),
                user: taker,
                reason: ReasonForOrderRemoval::SelfTradePrevention,
            })?;

            continue;
        }

        // ---------------------- Determine fillable size ----------------------

        let opposite = maker_order.size.checked_neg()?;

        let taker_fill_size = if taker_is_bid {
            remaining_size.min(opposite)
        } else {
            remaining_size.max(opposite)
        };

        let maker_fill_size = taker_fill_size.checked_neg()?;

        // ------------------------ Settle taker's PnL -------------------------

        let old_taker_pos = taker_state.positions.get(pair_id).cloned();

        settle_fill(
            pair_id,
            &mut pair_state,
            &mut taker_state,
            taker,
            taker_fill_size,
            resting_price,
            taker_fee_rate,
            &mut pnls,
            &mut fees,
            &mut volumes,
            Some((events, taker_order_id)),
        )?;

        if let Some(diff) = compute_position_diff(
            pair_id,
            taker,
            old_taker_pos.as_ref(),
            taker_state.positions.get(pair_id),
        ) {
            index_updates.push(diff);
        }

        // ------------------------ Settle maker's PnL -------------------------

        // Find the maker's user state.
        let maker_state = match maker_states.entry(maker_order.user) {
            Entry::Vacant(e) => {
                let maybe_maker_state = USER_STATES.may_load(storage, maker_order.user)?;
                e.insert(maybe_maker_state.unwrap_or_default())
            },
            Entry::Occupied(e) => e.into_mut(),
        };

        let old_maker_pos = maker_state.positions.get(pair_id).cloned();

        // Resolve maker's fee rate based on recent volume.
        let maker_fee_rate = {
            let maker_volume = query_volume(storage, maker_order.user, volume_since)?;
            param.maker_fee_rates.resolve(maker_volume)
        };

        settle_fill(
            pair_id,
            &mut pair_state,
            maker_state,
            maker_order.user,
            maker_fill_size,
            resting_price,
            maker_fee_rate,
            &mut pnls,
            &mut fees,
            &mut volumes,
            Some((events, maker_order_id)),
        )?;

        if let Some(diff) = compute_position_diff(
            pair_id,
            maker_order.user,
            old_maker_pos.as_ref(),
            maker_state.positions.get(pair_id),
        ) {
            index_updates.push(diff);
        }

        // ------------- Apply maker's child orders after fill -----------------

        if (maker_order.tp.is_some() || maker_order.sl.is_some())
            && let Some(maker_pos) = maker_state.positions.get_mut(pair_id)
            && maker_pos.size.is_positive() == maker_order.size.is_positive()
        {
            let (above, below) = map_child_orders(
                maker_pos.size,
                &maker_order.tp,
                &maker_order.sl,
                &mut next_order_id,
            );

            maker_pos.conditional_order_above = above;
            maker_pos.conditional_order_below = below;

            emit_child_order_events(
                events,
                pair_id,
                maker_order.user,
                &maker_order.tp,
                &maker_order.sl,
                maker_pos.size,
            )?;
        }

        // ---------------- Update maker's order and user state ----------------

        let pre_fill_abs_size = maker_order.size.checked_abs()?;

        // Release reserved margin proportionally to the filled portion.
        let margin_to_release = (maker_order.reserved_margin)
            .checked_mul(maker_fill_size)?
            .checked_div(maker_order.size)?;

        maker_state
            .reserved_margin
            .checked_sub_assign(margin_to_release)?;

        maker_order
            .reserved_margin
            .checked_sub_assign(margin_to_release)?;

        maker_order.size.checked_sub_assign(maker_fill_size)?;

        if maker_order.size.is_zero() {
            maker_state.open_order_count -= 1;

            order_mutations.push((stored_price, maker_order_id, None, pre_fill_abs_size));

            // Vault order removal is internal churn — suppress the event.
            if maker_order.user != contract {
                events.push(OrderRemoved {
                    order_id: maker_order_id,
                    pair_id: pair_id.clone(),
                    user: maker_order.user,
                    reason: ReasonForOrderRemoval::Filled,
                })?;
            }

            #[cfg(feature = "metrics")]
            {
                metrics::counter!(
                    crate::metrics::LABEL_ORDERS_FILLED,
                    "pair_id" => pair_id.to_string()
                )
                .increment(1);
            }
        } else {
            order_mutations.push((
                stored_price,
                maker_order_id,
                Some(maker_order),
                pre_fill_abs_size,
            ));
        }

        remaining_size.checked_sub_assign(taker_fill_size)?;
    }

    Ok(MatchOrderOutcome {
        pair_state,
        taker_state,
        maker_states,
        unfilled: remaining_size,
        pnls,
        fees,
        volumes,
        order_mutations,
        index_updates,
        next_order_id,
    })
}

/// Mutates:
///
/// - `pair_state.long_oi` / `pair_state.short_oi` — updated by `execute_fill`.
/// - `user_state.positions` — opened / closed / flipped by `execute_fill`.
/// - `pnls` — position PnL added for `user`.
/// - `fees` — trading fee added for `user`.
/// - `events` — `OrderFilled` event pushed (if `Some`).
pub fn settle_fill(
    pair_id: &PairId,
    pair_state: &mut PairState,
    user_state: &mut UserState,
    user: Addr,
    fill_size: Quantity,
    fill_price: UsdPrice,
    fee_rate: Dimensionless,
    pnls: &mut BTreeMap<Addr, UsdValue>,
    fees: &mut BTreeMap<Addr, UsdValue>,
    volumes: &mut BTreeMap<Addr, UsdValue>,
    events: Option<(&mut EventBuilder, OrderId)>,
) -> grug::StdResult<UsdValue> {
    let (closing, opening) = {
        let current_pos = user_state
            .positions
            .get(pair_id)
            .map(|p| p.size)
            .unwrap_or_default();
        decompose_fill(fill_size, current_pos)
    };

    let pnl = execute_fill(
        pair_id, pair_state, user_state, fill_price, closing, opening,
    )?;

    let fee = compute_trading_fee(fill_size, fill_price, fee_rate)?;

    let volume = compute_notional(fill_size, fill_price)?;

    pnls.entry(user).or_default().checked_add_assign(pnl)?;

    fees.entry(user).or_default().checked_add_assign(fee)?;

    volumes
        .entry(user)
        .or_default()
        .checked_add_assign(volume)?;

    if let Some((events, order_id)) = events {
        events.push(OrderFilled {
            order_id,
            pair_id: pair_id.clone(),
            user,
            fill_price,
            fill_size,
            closing_size: closing,
            opening_size: opening,
            realized_pnl: pnl,
            fee,
        })?;
    }

    #[cfg(feature = "metrics")]
    {
        let pair_label = pair_id.to_string();

        metrics::counter!(
            crate::metrics::LABEL_TRADES,
            "pair_id" => pair_label.clone()
        )
        .increment(1);

        let vol = volume.to_f64().abs();

        metrics::histogram!(
            crate::metrics::LABEL_VOLUME_PER_TRADE,
            "pair_id" => pair_label.clone()
        )
        .record(vol);

        metrics::histogram!(
            crate::metrics::LABEL_FEES_COLLECTED,
            "pair_id" => pair_label
        )
        .record(fee.to_f64().abs());
    }

    Ok(pnl)
}

#[derive(Debug)]
pub struct FeeBreakdown {
    /// Portion of the fee routed to the protocol treasury.
    pub protocol_fee: UsdValue,

    /// Portion of the fee credited to the vault.
    pub vault_fee: UsdValue,
}

/// Settle PnLs and fees directly in USD on user margins.
///
/// Two loops:
///
/// 1. **Fee loop** (first): non-vault fees increase the vault's margin (via its
///    `UserState`) and are deducted from the user's margin. Vault fees are
///    skipped (paying yourself is a no-op).
/// 2. **PnL loop** (second): each user's (including the vault's) margin is
///    adjusted by their PnL.
///
/// The taker is passed separately from `maker_states` because self-trade
/// prevention guarantees the taker never appears as a maker.
///
/// Mutates:
///
/// - `taker_state.margin` — adjusted by the taker's PnL and fees.
/// - `maker_states[*].margin` — adjusted by PnL and fees (including the vault's
///   `UserState`).
///
/// Per-user fee breakdown after splitting between protocol treasury and vault.
///
/// Returns: per-user fee breakdown — the split between protocol treasury and
/// vault for each fee-paying user.
pub fn settle_pnls(
    contract: Addr,
    param: &Param,
    state: &mut State,
    taker: Addr,
    taker_state: &mut UserState,
    maker_states: &mut BTreeMap<Addr, UserState>,
    pnls: BTreeMap<Addr, UsdValue>,
    fees: BTreeMap<Addr, UsdValue>,
) -> anyhow::Result<BTreeMap<Addr, FeeBreakdown>> {
    debug_assert!(
        !maker_states.contains_key(&taker),
        "taker must not be in maker_states — self-trade prevention violated"
    );

    // ------------------------------ Settle fees ------------------------------

    let mut fee_breakdowns = BTreeMap::new();

    for (user, fee) in fees {
        if fee.is_zero() || user == contract {
            continue;
        }

        // Split the fee between the protocol treasury and the vault.
        let protocol_fee = fee.checked_mul(param.protocol_fee_rate)?;
        let vault_fee = fee.checked_sub(protocol_fee)?;

        // Protocol treasury accumulates its share in global state.
        state.treasury.checked_add_assign(protocol_fee)?;

        // Vault receives its share.
        maker_states
            .get_mut(&contract)
            .unwrap()
            .margin
            .checked_add_assign(vault_fee)?;

        // Deduct fee from the paying user.
        if user == taker {
            taker_state.margin.checked_sub_assign(fee)?;
        } else {
            maker_states
                .get_mut(&user)
                .unwrap()
                .margin
                .checked_sub_assign(fee)?;
        }

        fee_breakdowns.insert(user, FeeBreakdown {
            protocol_fee,
            vault_fee,
        });
    }

    // ------------------------------ Settle PnLs ------------------------------

    for (user, pnl) in pnls {
        if pnl.is_zero() {
            continue;
        }

        if user == taker {
            taker_state.margin.checked_add_assign(pnl)?;
        } else {
            maker_states
                .get_mut(&user)
                .unwrap()
                .margin
                .checked_add_assign(pnl)?;
        }
    }

    Ok(fee_breakdowns)
}

/// Validate and store a post-only limit order. Rejects if the limit price
/// would cross the best resting order on the opposite side of the book.
/// Pure w.r.t. the caller's `UserState` — delegates to `store_limit_order`
/// and returns the same `StoreLimitOrderOutcome`.
fn store_post_only_limit_order(
    storage: &dyn Storage,
    taker: Addr,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    pair_id: &PairId,
    pair_param: &PairParam,
    taker_state: &UserState,
    size: Quantity,
    limit_price: UsdPrice,
    reduce_only: bool,
    order_id: OrderId,
    tp: Option<ChildOrder>,
    sl: Option<ChildOrder>,
) -> anyhow::Result<StoreLimitOrderOutcome> {
    let taker_is_bid = size.is_positive();
    let maker_is_bid = !taker_is_bid;

    let maker_book = if taker_is_bid {
        ASKS
    } else {
        BIDS
    };

    if let Some(record) = maker_book
        .prefix(pair_id.clone())
        .range(storage, None, None, IterationOrder::Ascending)
        .next()
    {
        let ((stored_price, _), _) = record?;
        let best_price = may_invert_price(stored_price, maker_is_bid);

        if taker_is_bid {
            ensure!(
                limit_price < best_price,
                "post-only buy at {limit_price} would cross best ask at {best_price}"
            );
        } else {
            ensure!(
                limit_price > best_price,
                "post-only sell at {limit_price} would cross best bid at {best_price}"
            );
        }
    }

    store_limit_order(
        storage,
        taker,
        current_time,
        oracle_querier,
        param,
        pair_param,
        taker_state,
        size,
        limit_price,
        reduce_only,
        order_id,
        tp,
        sl,
    )
}

/// Owned outcome of a `store_limit_order` call (and, by extension, of
/// `store_post_only_limit_order`, which delegates). Returned instead of
/// mutating the caller's `&mut UserState` so that a failed store leaves
/// the caller's state untouched.
#[derive(Debug)]
pub struct StoreLimitOrderOutcome {
    pub user_state: UserState,
    pub stored_price: UsdPrice,
    pub order_id: OrderId,
    pub order: LimitOrder,
}

/// Validate the caller, compute the reserved margin, and produce an
/// updated `UserState` with the new resting order accounted for (incremented
/// `open_order_count`, added `reserved_margin`). Pure w.r.t. the caller's
/// `UserState` — takes `&UserState` and returns the updated copy in the
/// outcome, so a failed call leaves the caller's state untouched.
///
/// The caller is responsible for persisting `outcome.user_state` back to
/// `USER_STATES` and writing the returned `order` to `BIDS` / `ASKS`.
fn store_limit_order(
    storage: &dyn Storage,
    user: Addr,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    pair_param: &PairParam,
    user_state: &UserState,
    size: Quantity,
    limit_price: UsdPrice,
    reduce_only: bool,
    order_id: OrderId,
    tp: Option<ChildOrder>,
    sl: Option<ChildOrder>,
) -> anyhow::Result<StoreLimitOrderOutcome> {
    ensure!(
        user_state.open_order_count < param.max_open_orders,
        "too many open orders! max allowed: {}",
        param.max_open_orders
    );

    // Enforce tick size: `limit_price` must be an integer multiple of `tick_size`.
    if pair_param.tick_size.is_non_zero() {
        ensure!(
            limit_price.checked_rem(pair_param.tick_size)?.is_zero(),
            "limit price ({}) is not a multiple of tick size ({})",
            limit_price,
            pair_param.tick_size,
        );
    }

    // Reserve margin for worst case (entire order is opening).
    let margin_to_reserve = compute_required_margin(size, limit_price, pair_param)?;

    // 0%-fill margin check: verify the user can afford this reservation.
    if !reduce_only {
        let perp_querier = NoCachePerpQuerier::new_local(storage);

        let available_margin = compute_available_margin(oracle_querier, &perp_querier, user_state)?;

        ensure!(
            available_margin >= margin_to_reserve,
            "insufficient margin for limit order: available ({}) < required ({})",
            available_margin,
            margin_to_reserve
        );
    }

    // Clone the user state and mutate the local copy. On `Err` the clone is
    // dropped with the rest of the call frame; the caller's `&UserState`
    // is never touched.
    let mut user_state = user_state.clone();
    user_state.open_order_count += 1;
    user_state
        .reserved_margin
        .checked_add_assign(margin_to_reserve)?;

    // Invert price for buy orders so storage order matches price-time priority.
    let stored_price = may_invert_price(limit_price, size.is_positive());

    Ok(StoreLimitOrderOutcome {
        user_state,
        stored_price,
        order_id,
        order: LimitOrder {
            user,
            size,
            reduce_only,
            reserved_margin: margin_to_reserve,
            created_at: current_time,
            tp,
            sl,
        },
    })
}

/// Map TP/SL child order params to above/below conditional orders based on
/// position direction, allocating order IDs from the shared counter.
///
/// - Long positions: TP → Above, SL → Below
/// - Short positions: TP → Below, SL → Above
fn map_child_orders(
    position_size: Quantity,
    tp: &Option<ChildOrder>,
    sl: &Option<ChildOrder>,
    next_order_id: &mut OrderId,
) -> (Option<ConditionalOrder>, Option<ConditionalOrder>) {
    let make_conditional = |child: &ChildOrder, next_id: &mut OrderId| -> ConditionalOrder {
        let order_id = *next_id;
        *next_id += OrderId::ONE;

        ConditionalOrder {
            order_id,
            size: child.size,
            trigger_price: child.trigger_price,
            max_slippage: child.max_slippage,
        }
    };

    let is_long = position_size.is_positive();

    // Map TP/SL to above/below based on direction.
    let (above_src, below_src) = if is_long {
        (tp, sl)
    } else {
        (sl, tp)
    };

    let above = above_src
        .as_ref()
        .map(|c| make_conditional(c, next_order_id));
    let below = below_src
        .as_ref()
        .map(|c| make_conditional(c, next_order_id));

    (above, below)
}

/// Emit `ConditionalOrderPlaced` events for child orders that were applied.
fn emit_child_order_events(
    events: &mut EventBuilder,
    pair_id: &PairId,
    user: Addr,
    tp: &Option<ChildOrder>,
    sl: &Option<ChildOrder>,
    position_size: Quantity,
) -> anyhow::Result<()> {
    let is_long = position_size.is_positive();

    if let Some(child) = tp {
        let direction = if is_long {
            TriggerDirection::Above
        } else {
            TriggerDirection::Below
        };

        events.push(ConditionalOrderPlaced {
            pair_id: pair_id.clone(),
            user,
            trigger_price: child.trigger_price,
            trigger_direction: direction,
            size: child.size,
            max_slippage: child.max_slippage,
        })?;
    }

    if let Some(child) = sl {
        let direction = if is_long {
            TriggerDirection::Below
        } else {
            TriggerDirection::Above
        };

        events.push(ConditionalOrderPlaced {
            pair_id: pair_id.clone(),
            user,
            trigger_price: child.trigger_price,
            trigger_direction: direction,
            size: child.size,
            max_slippage: child.max_slippage,
        })?;
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::USER_STATES,
        dango_types::{
            Dimensionless, FundingPerUnit,
            oracle::PrecisionedPrice,
            perps::{Position, RateSchedule},
        },
        grug::{Coins, MockContext, Timestamp, Udec128, Uint64, hash_map},
    };

    const CONTRACT: Addr = Addr::mock(0);
    const TAKER: Addr = Addr::mock(1);
    const MAKER_A: Addr = Addr::mock(2);
    const MAKER_B: Addr = Addr::mock(3);

    /// Large collateral value that trivially satisfies any margin check.
    const LARGE_COLLATERAL: UsdValue = UsdValue::new_int(999_999_999);

    fn test_oracle_querier() -> OracleQuerier<'static> {
        OracleQuerier::new_mock(hash_map! {
            pair_id() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        })
    }

    fn pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn test_param() -> Param {
        Param {
            max_open_orders: 10,
            taker_fee_rates: RateSchedule {
                base: Dimensionless::new_permille(1), // 0.1%
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn test_pair_param() -> PairParam {
        PairParam {
            max_abs_oi: Quantity::new_int(1_000_000),
            tick_size: UsdPrice::new_int(1),
            initial_margin_ratio: Dimensionless::new_permille(50), // 5%
            maintenance_margin_ratio: Dimensionless::new_permille(25), // 2.5%
            ..Default::default()
        }
    }

    fn setup_storage(storage: &mut dyn Storage) {
        PARAM.save(storage, &test_param()).unwrap();
        PAIR_PARAMS
            .save(storage, &pair_id(), &test_pair_param())
            .unwrap();
        PAIR_STATES
            .save(storage, &pair_id(), &PairState::default())
            .unwrap();
        NEXT_ORDER_ID.save(storage, &Uint64::new(1)).unwrap();
    }

    /// Place a resting ask (sell) order on the book.
    fn place_ask(storage: &mut dyn Storage, maker: Addr, price: i128, size: i128, order_id: u64) {
        let key = (pair_id(), UsdPrice::new_int(price), Uint64::new(order_id));
        let order = LimitOrder {
            user: maker,
            size: Quantity::new_int(-size.abs()),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(size.abs() * price / 20), // 5% margin
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
        };
        ASKS.save(storage, key, &order).unwrap();

        let mut maker_state = USER_STATES
            .may_load(storage, maker)
            .unwrap()
            .unwrap_or_default();
        maker_state.open_order_count += 1;
        maker_state
            .reserved_margin
            .checked_add_assign(order.reserved_margin)
            .unwrap();
        USER_STATES.save(storage, maker, &maker_state).unwrap();
    }

    /// Place a resting bid (buy) order on the book.
    fn place_bid(storage: &mut dyn Storage, maker: Addr, price: i128, size: i128, order_id: u64) {
        let inverted_price = !UsdPrice::new_int(price);
        let key = (pair_id(), inverted_price, Uint64::new(order_id));
        let order = LimitOrder {
            user: maker,
            size: Quantity::new_int(size.abs()),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(size.abs() * price / 20),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
        };
        BIDS.save(storage, key, &order).unwrap();

        let mut maker_state = USER_STATES
            .may_load(storage, maker)
            .unwrap()
            .unwrap_or_default();
        maker_state.open_order_count += 1;
        maker_state
            .reserved_margin
            .checked_add_assign(order.reserved_margin)
            .unwrap();
        USER_STATES.save(storage, maker, &maker_state).unwrap();
    }

    // =================== Market buy: single fill, full fill ==================

    #[test]
    fn market_buy_single_full_fill() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state,
            taker_state,
            order_mutations,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100), // 10%
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Taker should have a long position.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_000));

        // No order stored (market order, fully filled).
        assert!(order_to_store.is_none());

        // OI updated.
        assert_eq!(pair_state.long_oi, Quantity::new_int(10));

        // Ask should be removed from book (1 removal mutation).
        assert_eq!(order_mutations.len(), 1);
        assert!(order_mutations[0].2.is_none());
    }

    // ============= Market buy: partial fill (IOC cancels remainder) ===========

    #[test]
    fn market_buy_partial_fill_ioc() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Taker gets 5 (partial), remainder canceled (IOC).
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(5));

        assert!(order_to_store.is_none());
    }

    // ============= Market buy: no liquidity at acceptable price ==============

    #[test]
    fn market_buy_no_liquidity_errors() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("no liquidity at acceptable price")
        );
    }

    // ============= Limit buy: fully fills against book =======================

    #[test]
    fn limit_buy_fully_fills() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
                post_only: false,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));

        assert!(order_to_store.is_none());
    }

    // ============ Limit buy: partial fill, remainder stored as GTC ============

    #[test]
    fn limit_buy_partial_fill_remainder_stored() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
                post_only: false,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // 5 filled.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(5));

        // Remainder (5) stored as GTC.
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(5));
    }

    // ====== Limit buy: no matchable orders, entire order stored ==============

    #[test]
    fn limit_buy_no_match_stored_entirely() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 51_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
                post_only: false,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // No position (nothing filled).
        assert!(!taker_state.positions.contains_key(&pair_id()));

        // Entire order stored.
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(10));
    }

    // ================== Reduce-only: only closes existing ====================

    #[test]
    fn reduce_only_closes_only() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_bid(&mut ctx.storage, MAKER_A, 48_000, 20, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        pair_state.long_oi = Quantity::new_int(5);

        let mut taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        taker_state.positions.insert(pair_id(), Position {
            size: Quantity::new_int(5),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        });
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            true,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Position fully closed.
        assert!(!taker_state.positions.contains_key(&pair_id()));
        assert!(order_to_store.is_none());
    }

    // ============= Reduce-only with no position errors =======================

    #[test]
    fn reduce_only_no_position_errors() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10),
            },
            true,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("fillable size is zero")
        );
    }

    // ============= Market sell against resting bids ==========================

    #[test]
    fn market_sell_against_bids() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_bid(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state,
            taker_state,
            order_mutations,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Taker gets a short position.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(-10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_000));

        assert!(order_to_store.is_none());
        assert_eq!(pair_state.short_oi, Quantity::new_int(10));

        // Bid removed from book (1 removal mutation).
        assert_eq!(order_mutations.len(), 1);
        assert!(order_mutations[0].2.is_none());
    }

    // =========== Two-sided settlement: both positions updated =================

    #[test]
    fn two_sided_settlement() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Taker: long 10 @ 50000
        let taker_pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(taker_pos.size, Quantity::new_int(10));

        // Maker: short 10 @ 50000
        let maker_pos = maker_states[&MAKER_A].positions.get(&pair_id()).unwrap();
        assert_eq!(maker_pos.size, Quantity::new_int(-10));
        assert_eq!(maker_pos.entry_price, UsdPrice::new_int(50_000));
    }

    // ============= Fee accounting: net PnLs include fees =====================

    #[test]
    fn pnls_include_fees() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Taker: no realized PnL (opening), fee = |10| * 50000 * 0.001 = $500.
        // Margin decreases by $500.
        assert_eq!(
            taker_state.margin,
            LARGE_COLLATERAL
                .checked_sub(UsdValue::new_int(500))
                .unwrap()
        );

        // Fee goes to vault.
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(500));
    }

    // ======== Tick size enforcement for limit orders =========================

    #[test]
    fn tick_size_valid_multiple_accepted() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        let param = test_param();
        let mut pair_param = test_pair_param();
        pair_param.tick_size = UsdPrice::new_int(100);
        PAIR_PARAMS
            .save(&mut ctx.storage, &pair_id(), &pair_param)
            .unwrap();

        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        // 50,100 is a valid multiple of tick size 100 — should succeed.
        let result = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_100),
                post_only: false,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn tick_size_enforcement() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        let param = test_param();
        let mut pair_param = test_pair_param();
        pair_param.tick_size = UsdPrice::new_int(100);
        PAIR_PARAMS
            .save(&mut ctx.storage, &pair_id(), &pair_param)
            .unwrap();

        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_050),
                post_only: false,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("not a multiple of tick size")
        );
    }

    // ======= Multi-level fill: walks through price levels ====================

    #[test]
    fn market_buy_walks_price_levels() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);
        place_ask(&mut ctx.storage, MAKER_B, 50_100, 5, 101);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            order_mutations,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_100),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));

        // Weighted avg entry: (5*50000 + 5*50100) / 10 = 50050
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_050));

        assert!(order_to_store.is_none());

        // Both asks fully filled (2 removal mutations).
        assert_eq!(order_mutations.len(), 2);
        assert!(order_mutations[0].2.is_none());
        assert!(order_mutations[1].2.is_none());
    }

    // ======= Maker reserved margin release: full fill ========================

    #[test]
    fn maker_reserved_margin_released_on_full_fill() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let maker_state_before = USER_STATES.load(&ctx.storage, MAKER_A).unwrap();
        assert!(maker_state_before.reserved_margin.is_non_zero());
        assert_eq!(maker_state_before.open_order_count, 1);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        assert_eq!(maker_states[&MAKER_A].reserved_margin, UsdValue::ZERO);
        assert_eq!(maker_states[&MAKER_A].open_order_count, 0);
    }

    // ======= Maker reserved margin release: partial fill =====================

    #[test]
    fn maker_reserved_margin_released_on_partial_fill() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(4),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        assert_eq!(maker_states[&MAKER_A].open_order_count, 1);

        // initial_margin = 10 * 50000 / 20 = 25000 USD
        // 40% released, 60% remaining = 15000
        assert_eq!(
            maker_states[&MAKER_A].reserved_margin,
            UsdValue::new_int(15_000)
        );
    }

    // ======= Market buy: price beyond slippage ===============================

    #[test]
    fn market_buy_slippage_exceeded() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 60_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(10), // 1%
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(err.is_err());
        assert!(
            err.unwrap_err()
                .to_string()
                .contains("no liquidity at acceptable price")
        );
    }

    // =================== settle_pnls unit tests ==============================

    #[test]
    fn settle_pnls_mixed() {
        let taker = Addr::mock(1);
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([
            (CONTRACT, UserState::default()),
            (Addr::mock(2), UserState::default()),
            (Addr::mock(3), UserState::default()),
        ]);

        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(100)),
            (Addr::mock(2), UsdValue::new_int(-200)),
            (Addr::mock(3), UsdValue::ZERO),
        ]);
        let fees = BTreeMap::new();
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        // Positive PnL: taker margin += $100.
        assert_eq!(taker_state.margin, UsdValue::new_int(100));

        // Negative PnL: user 2 margin -= $200.
        assert_eq!(maker_states[&Addr::mock(2)].margin, UsdValue::new_int(-200));

        // Zero PnL: user 3 margin unchanged.
        assert_eq!(maker_states[&Addr::mock(3)].margin, UsdValue::ZERO);

        // Non-vault PnL does not change vault margin.
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::ZERO);
    }

    #[test]
    fn settle_pnls_all_payouts() {
        let taker = Addr::mock(1);
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([
            (CONTRACT, UserState::default()),
            (Addr::mock(2), UserState::default()),
        ]);

        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(100)),
            (Addr::mock(2), UsdValue::new_int(50)),
        ]);
        let fees = BTreeMap::new();
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        assert_eq!(taker_state.margin, UsdValue::new_int(100));
        assert_eq!(maker_states[&Addr::mock(2)].margin, UsdValue::new_int(50));

        // Non-vault PnL does not change vault margin.
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::ZERO);
    }

    #[test]
    fn settle_pnls_all_collections() {
        let taker = Addr::mock(1);
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([
            (CONTRACT, UserState::default()),
            (Addr::mock(2), UserState::default()),
        ]);

        let pnls = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(-100)),
            (Addr::mock(2), UsdValue::new_int(-200)),
        ]);
        let fees = BTreeMap::new();
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        assert_eq!(taker_state.margin, UsdValue::new_int(-100));
        assert_eq!(maker_states[&Addr::mock(2)].margin, UsdValue::new_int(-200));

        // Non-vault PnL does not change vault margin.
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::ZERO);
    }

    #[test]
    fn settle_pnls_empty() {
        let taker = TAKER;
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([(CONTRACT, UserState {
            margin: UsdValue::new_int(500),
            ..Default::default()
        })]);

        let pnls = BTreeMap::new();
        let fees = BTreeMap::new();
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(500));
    }

    #[test]
    fn settle_pnls_fees_increase_vault_margin() {
        let taker = Addr::mock(1);
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([
            (CONTRACT, UserState::default()),
            (Addr::mock(2), UserState::default()),
        ]);

        let pnls = BTreeMap::new();
        let fees = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(50)),
            (Addr::mock(2), UsdValue::new_int(100)),
        ]);
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        // Taker's margin decreases by fee amount.
        assert_eq!(taker_state.margin, UsdValue::new_int(-50));

        // Maker's margin decreases by fee amount.
        assert_eq!(maker_states[&Addr::mock(2)].margin, UsdValue::new_int(-100));

        // Fees go to vault margin: $50 + $100 = $150.
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(150));
    }

    #[test]
    fn settle_pnls_vault_pnl_adjusts_margin() {
        let taker = TAKER;
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([(CONTRACT, UserState {
            margin: UsdValue::new_int(1_000),
            ..Default::default()
        })]);

        // Vault profit of $500.
        let pnls = BTreeMap::from([(CONTRACT, UsdValue::new_int(500))]);
        let fees = BTreeMap::new();
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(1_500));
    }

    #[test]
    fn settle_pnls_vault_loss_creates_bad_debt() {
        let taker = TAKER;
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([(CONTRACT, UserState {
            margin: UsdValue::new_int(100),
            ..Default::default()
        })]);

        // Vault loss of $500.
        let pnls = BTreeMap::from([(CONTRACT, UsdValue::new_int(-500))]);
        let fees = BTreeMap::new();
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(-400));
    }

    #[test]
    fn settle_pnls_vault_profit_recovers_negative_margin() {
        let taker = TAKER;
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([(CONTRACT, UserState {
            margin: UsdValue::new_int(-300),
            ..Default::default()
        })]);

        // Vault profit of $500.
        let pnls = BTreeMap::from([(CONTRACT, UsdValue::new_int(500))]);
        let fees = BTreeMap::new();
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        // Vault margin recovers: -300 + 500 = 200.
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(200));
    }

    #[test]
    fn settle_pnls_vault_fees_skipped() {
        let taker = TAKER;
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([(CONTRACT, UserState {
            margin: UsdValue::new_int(1_000),
            ..Default::default()
        })]);

        // Vault's own fees are a no-op (paying yourself).
        let pnls = BTreeMap::new();
        let fees = BTreeMap::from([(CONTRACT, UsdValue::new_int(100))]);
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(),
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(1_000));
    }

    #[test]
    fn settle_pnls_protocol_fee_split() {
        let taker = Addr::mock(1);
        let mut taker_state = UserState::default();
        let param = Param {
            protocol_fee_rate: Dimensionless::new_percent(20),
            ..Default::default()
        };
        let mut maker_states = BTreeMap::from([(CONTRACT, UserState::default())]);
        let pnls = BTreeMap::new();
        let fees = BTreeMap::from([(Addr::mock(1), UsdValue::new_int(100))]);
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &param,
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        assert_eq!(taker_state.margin, UsdValue::new_int(-100));
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(80));
        assert_eq!(state.treasury, UsdValue::new_int(20));
    }

    // =========== Negative maker fee (rebate) settle_pnls tests ===============

    #[test]
    fn settle_pnls_negative_maker_fee_rebate() {
        let taker = Addr::mock(1);
        let mut taker_state = UserState::default();
        let mut maker_states = BTreeMap::from([
            (CONTRACT, UserState::default()),
            (Addr::mock(2), UserState::default()),
        ]);

        let pnls = BTreeMap::new();
        // Taker pays +$50 fee, maker receives -$10 fee (rebate).
        let fees = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(50)),
            (Addr::mock(2), UsdValue::new_int(-10)),
        ]);
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &Param::default(), // protocol_fee_rate = 0
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        // Taker margin decreases by fee.
        assert_eq!(taker_state.margin, UsdValue::new_int(-50));

        // Maker margin increases (rebate): 0 - (-10) = +10.
        assert_eq!(maker_states[&Addr::mock(2)].margin, UsdValue::new_int(10));

        // Vault receives net: +50 (from taker) + (-10) (rebate to maker) = +40.
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(40));

        // No protocol fee split → treasury unchanged.
        assert_eq!(state.treasury, UsdValue::ZERO);
    }

    #[test]
    fn settle_pnls_negative_maker_fee_with_protocol_split() {
        let taker = Addr::mock(1);
        let mut taker_state = UserState::default();
        let param = Param {
            protocol_fee_rate: Dimensionless::new_percent(20),
            ..Default::default()
        };
        let mut maker_states = BTreeMap::from([
            (CONTRACT, UserState::default()),
            (Addr::mock(2), UserState::default()),
        ]);

        let pnls = BTreeMap::new();
        // Taker fee = +$30, maker fee = -$10 (rebate).
        let fees = BTreeMap::from([
            (Addr::mock(1), UsdValue::new_int(30)),
            (Addr::mock(2), UsdValue::new_int(-10)),
        ]);
        let mut state = State::default();

        settle_pnls(
            CONTRACT,
            &param,
            &mut state,
            taker,
            &mut taker_state,
            &mut maker_states,
            pnls,
            fees,
        )
        .unwrap();

        // Taker pays full fee.
        assert_eq!(taker_state.margin, UsdValue::new_int(-30));

        // Maker gets rebate: 0 - (-10) = +10.
        assert_eq!(maker_states[&Addr::mock(2)].margin, UsdValue::new_int(10));

        // Vault receives:
        //   from taker: $30 * 80% = $24
        //   from maker: -$10 * 80% = -$8
        //   total = $16
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(16));

        // Treasury receives:
        //   from taker: $30 * 20% = $6
        //   from maker: -$10 * 20% = -$2
        //   total = $4
        assert_eq!(state.treasury, UsdValue::new_int(4));
    }

    // ====== Negative maker fee: full _submit_order integration ===============

    #[test]
    fn negative_maker_fee_rebate_on_market_buy() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        // Custom param: taker = 3 bps, maker = -1 bps, protocol = 20%.
        let param = Param {
            max_open_orders: 10,
            taker_fee_rates: RateSchedule {
                base: Dimensionless::new_raw(300), // 3 bps
                ..Default::default()
            },
            maker_fee_rates: RateSchedule {
                base: Dimensionless::new_raw(-100), // -1 bps
                ..Default::default()
            },
            protocol_fee_rate: Dimensionless::new_percent(20),
            ..Default::default()
        };

        // Manually set up storage (can't use setup_storage which saves test_param).
        PARAM.save(&mut ctx.storage, &param).unwrap();
        PAIR_PARAMS
            .save(&mut ctx.storage, &pair_id(), &test_pair_param())
            .unwrap();
        PAIR_STATES
            .save(&mut ctx.storage, &pair_id(), &PairState::default())
            .unwrap();
        NEXT_ORDER_ID
            .save(&mut ctx.storage, &Uint64::new(1))
            .unwrap();

        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let state = State::default();
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            state,
            pair_state: _,
            taker_state,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &state,
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Notional = 10 * $50,000 = $500,000.
        // Taker fee = $500,000 * 3 bps = $150.
        // Maker fee = $500,000 * (-1 bps) = -$50 (rebate).
        //
        // Taker margin: LARGE_COLLATERAL - $150.
        assert_eq!(
            taker_state.margin,
            LARGE_COLLATERAL
                .checked_sub(UsdValue::new_int(150))
                .unwrap()
        );

        // Maker margin: 0 - (-$50) = +$50 (receives rebate).
        assert_eq!(maker_states[&MAKER_A].margin, UsdValue::new_int(50));

        // Vault receives:
        //   taker vault_fee = $150 * 80% = $120
        //   maker vault_fee = -$50 * 80% = -$40
        //   total = $80
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(80));

        // Treasury:
        //   taker protocol_fee = $150 * 20% = $30
        //   maker protocol_fee = -$50 * 20% = -$10
        //   total = $20
        assert_eq!(state.treasury, UsdValue::new_int(20));

        // Positions are correct.
        let taker_pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(taker_pos.size, Quantity::new_int(10));
        assert_eq!(taker_pos.entry_price, UsdPrice::new_int(50_000));
    }

    // =================== Post-only order tests ===============================

    #[test]
    fn post_only_buy_rests_below_best_ask() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            order_mutations,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(49_000),
                post_only: true,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // No fills — order rests.
        assert!(!taker_state.positions.contains_key(&pair_id()));
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(10));

        // No mutations.
        assert!(order_mutations.is_empty());
    }

    #[test]
    fn post_only_buy_at_best_ask_rejected() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
                post_only: true,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("post-only buy"));
    }

    #[test]
    fn post_only_buy_above_best_ask_rejected() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(51_000),
                post_only: true,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("post-only buy"));
    }

    #[test]
    fn post_only_sell_rests_above_best_bid() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_bid(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            order_mutations,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(-10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(51_000),
                post_only: true,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // No fills — order rests.
        assert!(!taker_state.positions.contains_key(&pair_id()));
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(-10));

        assert!(order_mutations.is_empty());
    }

    #[test]
    fn post_only_sell_at_best_bid_rejected() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_bid(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(-10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
                post_only: true,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("post-only sell"));
    }

    #[test]
    fn post_only_buy_empty_book_succeeds() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        // No asks placed — empty book.

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(49_000),
                post_only: true,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(10));
    }

    #[test]
    fn post_only_reduce_only_rests() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_bid(&mut ctx.storage, MAKER_A, 48_000, 20, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        pair_state.long_oi = Quantity::new_int(5);

        let mut taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        taker_state.positions.insert(pair_id(), Position {
            size: Quantity::new_int(5),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        });
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(-5),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(51_000),
                post_only: true,
            },
            true,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Order rests, reduce_only flag preserved.
        assert!(order_to_store.is_some());
        let (_, _, order) = order_to_store.unwrap();
        assert_eq!(order.size, Quantity::new_int(-5));
        assert!(order.reduce_only);
    }

    // ======= Post-only insufficient margin rejected ==========================

    /// Post-only buy with insufficient collateral is rejected by the 0%-fill
    /// margin check inside `store_limit_order`.
    ///
    /// pair: BTC, oracle = $50,000, IMR = 5%
    /// Buy 10 BTC post-only @ $49,000
    ///   margin_to_reserve = |10| * 49,000 * 0.05 = $24,500
    ///   collateral = $1,000 → available = $1,000 < $24,500 → FAILS
    #[test]
    fn post_only_insufficient_margin_rejected() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: UsdValue::new_int(1_000), // insufficient collateral
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let err = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(49_000),
                post_only: true,
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        );

        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("insufficient margin for limit order"),
            "expected limit-order margin error, got: {msg}"
        );
    }

    // ======= Self-trade prevention (EXPIRE_MAKER) ========================

    #[test]
    fn self_trade_prevention_expire_maker() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        // TAKER has a resting ask at 50,000 (order_id 100).
        place_ask(&mut ctx.storage, TAKER, 50_000, 10, 100);
        // MAKER_A has a resting ask behind it at 50,100 (order_id 101).
        place_ask(&mut ctx.storage, MAKER_A, 50_100, 10, 101);

        // Snapshot taker state after placing the resting order.
        let taker_state_before = USER_STATES.load(&ctx.storage, TAKER).unwrap();
        assert_eq!(taker_state_before.open_order_count, 1);
        let taker_reserved_before = taker_state_before.reserved_margin;

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();

        // Start with the taker state from storage (has the resting order's
        // reserved margin and open_order_count).
        let mut taker_state = taker_state_before.clone();
        taker_state.margin = LARGE_COLLATERAL;
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state,
            maker_states,
            order_mutations,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(50_100),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100), // 10%
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // 1) Taker's own ask was cancelled (mutation = None).
        assert_eq!(order_mutations.len(), 2);
        assert!(
            order_mutations[0].2.is_none(),
            "taker's resting order should be removed"
        );

        // 2) Taker's open_order_count was decremented (the resting order
        //    was cancelled by STP).
        assert_eq!(taker_state.open_order_count, 0);

        // 3) Taker's reserved margin from the cancelled order was released.
        assert!(taker_reserved_before.is_non_zero());
        // The reserved_margin for the cancelled order is fully released.
        // Only trading-fee deductions (from the fill against MAKER_A) remain.
        assert!(taker_state.reserved_margin < taker_reserved_before);

        // 4) Taker DID fill against the second maker's ask at 50,100.
        let pos = taker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(10));
        assert_eq!(pos.entry_price, UsdPrice::new_int(50_100));

        // 5) MAKER_A's ask was fully filled (second mutation = None).
        assert!(
            order_mutations[1].2.is_none(),
            "MAKER_A's order should be fully filled"
        );
        assert_eq!(maker_states[&MAKER_A].open_order_count, 0);
    }

    // ======= Vault-as-maker PnL settlement ===================================

    /// Helper: run a two-step trade where the vault (CONTRACT) is the maker on
    /// both legs.
    ///
    /// Step 1: vault ask at `open_price` matched by taker buy → vault opens short.
    /// Step 2: vault bid at `close_price` matched by taker sell → vault closes short.
    ///
    /// Mutates: nothing persisted.
    ///
    /// Returns: the vault's margin (from its `UserState`) after both trades.
    fn vault_maker_round_trip(
        initial_vault_margin: UsdValue,
        open_price: i128,
        close_price: i128,
        size: i128,
    ) -> UsdValue {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        // Seed vault UserState with the initial margin.
        let vault_init = UserState {
            margin: initial_vault_margin,
            ..Default::default()
        };
        USER_STATES
            .save(&mut ctx.storage, CONTRACT, &vault_init)
            .unwrap();

        // ---- Step 1: vault places ask, taker buys → vault opens short ----

        place_ask(&mut ctx.storage, CONTRACT, open_price, size, 100);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state,
            taker_state,
            maker_states,
            order_mutations,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(open_price),
            Quantity::new_int(size),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Persist side-effects so step 2 can load them.
        PAIR_STATES
            .save(&mut ctx.storage, &pair_id(), &pair_state)
            .unwrap();
        USER_STATES
            .save(&mut ctx.storage, TAKER, &taker_state)
            .unwrap();
        for (addr, ms) in &maker_states {
            USER_STATES.save(&mut ctx.storage, *addr, ms).unwrap();
        }
        for (stored_price, order_id, mutation, _) in order_mutations {
            let key = (pair_id(), stored_price, order_id);
            match mutation {
                Some(order) => ASKS.save(&mut ctx.storage, key, &order).unwrap(),
                None => ASKS.remove(&mut ctx.storage, key).unwrap(),
            }
        }

        // Vault must now have a short position.
        let vault_state = USER_STATES.load(&ctx.storage, CONTRACT).unwrap();
        let vault_pos = vault_state.positions.get(&pair_id()).unwrap();
        assert_eq!(vault_pos.size, Quantity::new_int(-size.abs()));

        // ---- Step 2: vault places bid, taker sells → vault closes short ----

        place_bid(&mut ctx.storage, CONTRACT, close_price, size, 200);

        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = USER_STATES.load(&ctx.storage, TAKER).unwrap();
        let mut oq = test_oracle_querier();

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(close_price),
            Quantity::new_int(-size),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        maker_states[&CONTRACT].margin
    }

    /// Vault opens short at $50,000 then closes at $49,000 → profit of $10,000.
    ///
    /// Vault margin changes from: vault PnL + non-vault fees only.
    /// Non-vault PnL (taker's loss) does NOT change vault margin — the
    /// losing counterparty's margin is adjusted internally.
    #[test]
    fn vault_maker_realizes_profit() {
        let initial_margin = UsdValue::new_int(100_000);

        let vault_margin = vault_maker_round_trip(initial_margin, 50_000, 49_000, 10);

        // Vault margin tracks vault PnL + fee flows only:
        //
        // Step 1 (open at $50,000):
        //   taker fee = |10| × $50,000 × 0.001 = $500 → vault margin += $500
        //
        // Step 2 (close at $49,000):
        //   vault PnL = +$10,000 → vault margin += $10,000
        //   taker fee = $490 → vault margin += $490
        //   taker PnL = -$10,000 → taker margin adjusted (no vault margin change)
        //
        // Total Δ = $500 + $10,000 + $490 = $10,990
        assert_eq!(vault_margin, UsdValue::new_int(110_990));
    }

    /// Vault opens short at $50,000 then closes at $51,000 → loss of $10,000.
    /// Vault margin is large enough to absorb the loss entirely.
    #[test]
    fn vault_maker_realizes_loss_no_bad_debt() {
        let initial_margin = UsdValue::new_int(100_000);

        let vault_margin = vault_maker_round_trip(initial_margin, 50_000, 51_000, 10);

        // Vault margin tracks vault PnL + fee flows only:
        //
        // Step 1 (open at $50,000):
        //   taker fee = $500 → vault margin += $500
        //
        // Step 2 (close at $51,000):
        //   taker fee = $510 → vault margin += $510
        //   vault PnL = -$10,000 → vault margin -= $10,000
        //   taker PnL = +$10,000 → taker margin adjusted (no vault margin change)
        //
        // Total Δ = $500 + $510 - $10,000 = -$8,990
        assert_eq!(vault_margin, UsdValue::new_int(91_010));
    }

    /// Vault has an existing short position at $50,000. A new taker (MAKER_B)
    /// sells against the vault's bid at $51,000, closing the vault's short at a
    /// loss. Vault margin is only $1,000 — not enough to cover the $10,000 loss,
    /// so vault margin goes negative (representing the deficit).
    ///
    /// Fees are collected first (augmenting vault margin), then the vault loss
    /// is applied.
    #[test]
    fn vault_maker_realizes_loss_with_bad_debt() {
        let mut ctx = MockContext::new()
            .with_sender(MAKER_B)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        // Manually set up vault with a short position: -10 @ $50,000
        // and $1,000 margin.
        let mut vault_state = UserState {
            margin: UsdValue::new_int(1_000),
            ..Default::default()
        };
        vault_state.positions.insert(pair_id(), Position {
            size: Quantity::new_int(-10),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        });
        USER_STATES
            .save(&mut ctx.storage, CONTRACT, &vault_state)
            .unwrap();

        // Reflect existing OI from the vault's short.
        let mut pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        pair_state.short_oi = Quantity::new_int(10);
        PAIR_STATES
            .save(&mut ctx.storage, &pair_id(), &pair_state)
            .unwrap();

        // Vault places a resting bid at $51,000.
        place_bid(&mut ctx.storage, CONTRACT, 51_000, 10, 200);

        let param = test_param();
        let pair_param = test_pair_param();
        let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
        let taker_state = UserState {
            margin: LARGE_COLLATERAL,
            ..Default::default()
        };
        let mut oq = test_oracle_querier();

        // MAKER_B sells -10 → matches vault bid at $51,000.
        //   vault: closes short at $51,000 → loss = -$10,000
        //   MAKER_B: opens new short → PnL = 0, fee = |10| × $51,000 × 0.001 = $510
        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            MAKER_B,
            CONTRACT,
            Timestamp::ZERO,
            &mut oq,
            &param,
            &State::default(),
            &pair_id(),
            &pair_param,
            &pair_state,
            &taker_state,
            UsdPrice::new_int(51_000),
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Fee loop first: MAKER_B fee = $510 → vault margin += $510
        //   vault margin = $1,000 + $510 = $1,510
        //
        // PnL loop: vault loss = $10,000
        //   vault margin = $1,510 - $10,000 = -$8,490
        assert_eq!(maker_states[&CONTRACT].margin, UsdValue::new_int(-8_490));
    }

    // ===================== Regression: phantom order IDs =====================

    /// Previously, `NEXT_ORDER_ID` was only incremented when an order entered
    /// the book (GTC remainder or post-only). Market orders and limit orders
    /// that were fully filled immediately used a "phantom" order ID: the
    /// value of `NEXT_ORDER_ID` appeared in their `OrderFilled` events but
    /// was never actually consumed, so a subsequent order could reuse the
    /// same ID.
    ///
    /// After the fix, `_submit_order` always increments the order ID counter,
    /// regardless of whether the order enters the book.
    #[test]
    fn orders_not_entering_book_should_increment_next_order_id() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        let param = test_param();
        let pair_param = test_pair_param();

        // Case 1: Market order (fully filled, nothing enters book).
        {
            place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

            let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
            let taker_state = UserState {
                margin: LARGE_COLLATERAL,
                ..Default::default()
            };
            let mut oq = test_oracle_querier();

            let SubmitOrderOutcome {
                pair_state,
                taker_state: _,
                order_to_store,
                next_order_id,
                ..
            } = _submit_order(
                &ctx.storage,
                TAKER,
                CONTRACT,
                Timestamp::ZERO,
                &mut oq,
                &param,
                &State::default(),
                &pair_id(),
                &pair_param,
                &pair_state,
                &taker_state,
                UsdPrice::new_int(50_000),
                Quantity::new_int(10),
                OrderKind::Market {
                    max_slippage: Dimensionless::new_permille(100),
                },
                false,
                None,
                None,
                &mut EventBuilder::new(),
            )
            .unwrap();

            assert!(
                order_to_store.is_none(),
                "market order should not enter book"
            );
            assert_eq!(
                next_order_id,
                OrderId::new(2),
                "next_order_id must advance even when order doesn't enter book"
            );

            // Persist for case 2.
            NEXT_ORDER_ID
                .save(&mut ctx.storage, &next_order_id)
                .unwrap();
            PAIR_STATES
                .save(&mut ctx.storage, &pair_id(), &pair_state)
                .unwrap();

            // Clean up the filled ask.
            ASKS.remove(
                &mut ctx.storage,
                (pair_id(), UsdPrice::new_int(50_000), Uint64::new(100)),
            )
            .unwrap();

            // Place a fresh ask for case 2.
            place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 200);
        }

        // Case 2: Limit order that fully fills (nothing enters book).
        {
            let pair_state = PAIR_STATES.load(&ctx.storage, &pair_id()).unwrap();
            let taker_state = UserState {
                margin: LARGE_COLLATERAL,
                ..Default::default()
            };
            let mut oq = test_oracle_querier();

            let SubmitOrderOutcome {
                pair_state: _,
                taker_state: _,
                order_to_store,
                next_order_id,
                ..
            } = _submit_order(
                &ctx.storage,
                TAKER,
                CONTRACT,
                Timestamp::ZERO,
                &mut oq,
                &param,
                &State::default(),
                &pair_id(),
                &pair_param,
                &pair_state,
                &taker_state,
                UsdPrice::new_int(50_000),
                Quantity::new_int(10),
                OrderKind::Limit {
                    limit_price: UsdPrice::new_int(50_000),
                    post_only: false,
                },
                false,
                None,
                None,
                &mut EventBuilder::new(),
            )
            .unwrap();

            assert!(
                order_to_store.is_none(),
                "fully-filled limit should not enter book"
            );
            assert_eq!(
                next_order_id,
                OrderId::new(3),
                "next_order_id must advance again"
            );
        }
    }

    // ==================== Child order tests ====================

    fn make_tp(trigger_price: i128) -> Option<ChildOrder> {
        Some(ChildOrder {
            trigger_price: UsdPrice::new_int(trigger_price),
            max_slippage: Dimensionless::new_percent(1),
            size: None,
        })
    }

    fn make_sl(trigger_price: i128) -> Option<ChildOrder> {
        Some(ChildOrder {
            trigger_price: UsdPrice::new_int(trigger_price),
            max_slippage: Dimensionless::new_percent(2),
            size: None,
        })
    }

    /// Market buy with TP/SL → long position gets above (TP) and below (SL).
    #[test]
    fn co1_market_buy_applies_tp_sl() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            taker_state: ts, ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState::default(),
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            make_tp(55_000),
            make_sl(45_000),
            &mut EventBuilder::new(),
        )
        .unwrap();

        let pos = ts.positions.get(&pair_id()).unwrap();
        assert!(pos.size.is_positive(), "should be long");

        // TP → Above for long
        let above = pos.conditional_order_above.as_ref().unwrap();
        assert_eq!(above.trigger_price, UsdPrice::new_int(55_000));

        // SL → Below for long
        let below = pos.conditional_order_below.as_ref().unwrap();
        assert_eq!(below.trigger_price, UsdPrice::new_int(45_000));
    }

    /// Market sell with TP/SL → short position gets below (TP) and above (SL).
    #[test]
    fn co2_market_sell_applies_tp_sl() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);
        place_bid(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            taker_state: ts, ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState::default(),
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            make_tp(45_000),
            make_sl(55_000),
            &mut EventBuilder::new(),
        )
        .unwrap();

        let pos = ts.positions.get(&pair_id()).unwrap();
        assert!(pos.size.is_negative(), "should be short");

        // TP → Below for short
        let below = pos.conditional_order_below.as_ref().unwrap();
        assert_eq!(below.trigger_price, UsdPrice::new_int(45_000));

        // SL → Above for short
        let above = pos.conditional_order_above.as_ref().unwrap();
        assert_eq!(above.trigger_price, UsdPrice::new_int(55_000));
    }

    /// Market order that fully closes position → TP/SL dropped (position gone).
    #[test]
    fn co3_ignored_on_full_close() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);

        // Give taker an existing long position.
        let mut ts = taker_state(&ctx.storage);
        ts.positions.insert(pair_id(), Position {
            size: Quantity::new_int(10),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        });
        USER_STATES.save(&mut ctx.storage, TAKER, &ts).unwrap();

        // Place bid to absorb the sell.
        place_bid(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        // Sell 10 (closes the long). TP/SL should be dropped.
        let pair_state = PairState {
            long_oi: Quantity::new_int(10),
            ..Default::default()
        };

        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            taker_state: ts, ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &pair_state,
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(-10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            make_tp(45_000),
            make_sl(55_000),
            &mut EventBuilder::new(),
        )
        .unwrap();

        assert!(
            !ts.positions.contains_key(&pair_id()),
            "position should be closed"
        );
    }

    /// Order with only TP (no SL) → above is set, below is cleared.
    #[test]
    fn co4_tp_only_clears_sl() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            taker_state: ts, ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState::default(),
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            make_tp(55_000),
            None, // no SL
            &mut EventBuilder::new(),
        )
        .unwrap();

        let pos = ts.positions.get(&pair_id()).unwrap();
        assert!(pos.conditional_order_above.is_some(), "TP should be set");
        assert!(pos.conditional_order_below.is_none(), "SL should be None");
    }

    /// Limit order partially fills → TP/SL applied to position AND stored on
    /// resting order.
    #[test]
    fn co5_limit_partial_fill_applies_and_rests() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);

        // Only 5 available on the ask side, taker wants 10.
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);

        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            taker_state: ts,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState::default(),
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(50_000),
                post_only: false,
            },
            false,
            make_tp(55_000),
            make_sl(45_000),
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Position should have TP/SL from partial fill.
        let pos = ts.positions.get(&pair_id()).unwrap();
        assert!(pos.conditional_order_above.is_some());
        assert!(pos.conditional_order_below.is_some());

        // Resting order should also carry TP/SL.
        let (_, _, resting_order) = order_to_store.expect("should have resting order");
        assert!(resting_order.tp.is_some());
        assert!(resting_order.sl.is_some());
    }

    /// Limit order doesn't fill at all → TP/SL NOT applied to any existing
    /// position, only stored on the resting order.
    #[test]
    fn co6_no_fill_limit_not_applied() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);

        // Give taker an existing long with no conditional orders.
        let mut ts = taker_state(&ctx.storage);
        ts.positions.insert(pair_id(), Position {
            size: Quantity::new_int(5),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        });
        USER_STATES.save(&mut ctx.storage, TAKER, &ts).unwrap();

        // Place a buy limit at 49_000 — no asks below that so nothing fills.
        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            taker_state: ts,
            order_to_store,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState::default(),
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Limit {
                limit_price: UsdPrice::new_int(49_000),
                post_only: false,
            },
            false,
            make_tp(55_000),
            make_sl(45_000),
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Position should NOT have conditional orders (no fills).
        let pos = ts.positions.get(&pair_id()).unwrap();
        assert!(pos.conditional_order_above.is_none());
        assert!(pos.conditional_order_below.is_none());

        // Resting order carries TP/SL for later.
        let (_, _, resting_order) = order_to_store.expect("should have resting order");
        assert!(resting_order.tp.is_some());
        assert!(resting_order.sl.is_some());
    }

    /// Maker's resting limit order with TP/SL gets filled → TP/SL applied to
    /// maker's resulting position.
    #[test]
    fn co7_maker_fill_applies_tp_sl() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);

        // Place a maker ask with TP/SL child orders.
        let key = (pair_id(), UsdPrice::new_int(50_000), Uint64::new(100));
        let order = LimitOrder {
            user: MAKER_A,
            size: Quantity::new_int(-10),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(25_000),
            created_at: Timestamp::from_nanos(0),
            tp: make_tp(45_000),
            sl: make_sl(55_000),
        };
        ASKS.save(&mut ctx.storage, key, &order).unwrap();

        let maker_a_state = UserState {
            margin: LARGE_COLLATERAL,
            open_order_count: 1,
            reserved_margin: UsdValue::new_int(25_000),
            ..Default::default()
        };
        USER_STATES
            .save(&mut ctx.storage, MAKER_A, &maker_a_state)
            .unwrap();

        // Taker buys 10 → fills maker's ask.
        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState::default(),
            &taker_state(&ctx.storage),
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Maker should have a short position with TP/SL.
        let maker_state = maker_states.get(&MAKER_A).unwrap();
        let pos = maker_state.positions.get(&pair_id()).unwrap();
        assert!(pos.size.is_negative(), "maker should be short");

        // For shorts: TP → Below, SL → Above
        let below = pos.conditional_order_below.as_ref().unwrap();
        assert_eq!(below.trigger_price, UsdPrice::new_int(45_000));

        let above = pos.conditional_order_above.as_ref().unwrap();
        assert_eq!(above.trigger_price, UsdPrice::new_int(55_000));
    }

    /// Verify next_order_id is incremented correctly: taker order + child orders.
    #[test]
    fn co8_order_id_incremented() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 10, 100);

        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            next_order_id,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState::default(),
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            make_tp(55_000),
            make_sl(45_000),
            &mut EventBuilder::new(),
        )
        .unwrap();

        // ID 1 = taker order, ID 2 = TP child, ID 3 = SL child → next = 4
        assert_eq!(next_order_id, OrderId::new(4));
    }

    /// Position has existing conditional orders → new order with child orders
    /// overwrites both.
    #[test]
    fn co9_overwrites_existing_conditional_orders() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);

        // Give taker a long position with existing TP/SL.
        let mut ts = taker_state(&ctx.storage);
        ts.positions.insert(pair_id(), Position {
            size: Quantity::new_int(5),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: Some(ConditionalOrder {
                order_id: Uint64::new(99),
                size: None,
                trigger_price: UsdPrice::new_int(60_000),
                max_slippage: Dimensionless::new_percent(1),
            }),
            conditional_order_below: Some(ConditionalOrder {
                order_id: Uint64::new(98),
                size: None,
                trigger_price: UsdPrice::new_int(40_000),
                max_slippage: Dimensionless::new_percent(2),
            }),
        });
        USER_STATES.save(&mut ctx.storage, TAKER, &ts).unwrap();

        // Buy more with different TP/SL.
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);

        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            taker_state: ts, ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState {
                long_oi: Quantity::new_int(5),
                ..Default::default()
            },
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(5),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            make_tp(58_000), // different from existing 60k
            make_sl(42_000), // different from existing 40k
            &mut EventBuilder::new(),
        )
        .unwrap();

        let pos = ts.positions.get(&pair_id()).unwrap();

        let above = pos.conditional_order_above.as_ref().unwrap();
        assert_eq!(above.trigger_price, UsdPrice::new_int(58_000));
        assert_ne!(above.order_id, Uint64::new(99)); // new ID

        let below = pos.conditional_order_below.as_ref().unwrap();
        assert_eq!(below.trigger_price, UsdPrice::new_int(42_000));
        assert_ne!(below.order_id, Uint64::new(98)); // new ID
    }

    /// Taker has a short position but submits a buy with TP/SL (intended for
    /// long). Buy partially closes the short but doesn't flip it → direction
    /// mismatch → TP/SL dropped.
    ///
    /// Wrong behavior: applying TP/SL to the short position, which would
    /// place trigger prices in nonsensical slots (e.g., TP @ $55k as a Below
    /// trigger on a short would fire immediately).
    #[test]
    fn co10_taker_child_order_dropped_on_direction_mismatch() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);

        // Taker starts with a short position of -10.
        let ts = UserState {
            margin: LARGE_COLLATERAL,
            positions: {
                let mut p = BTreeMap::new();
                p.insert(pair_id(), Position {
                    size: Quantity::new_int(-10),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                });
                p
            },
            ..Default::default()
        };
        USER_STATES.save(&mut ctx.storage, TAKER, &ts).unwrap();

        // Place ask to fill the taker's buy.
        place_ask(&mut ctx.storage, MAKER_A, 50_000, 5, 100);

        let ts = taker_state(&ctx.storage);

        // Buy 5 with TP/SL intended for a long.
        let SubmitOrderOutcome {
            taker_state: ts, ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState {
                short_oi: Quantity::new_int(10),
                ..Default::default()
            },
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(5), // buy
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            make_tp(55_000),
            make_sl(45_000),
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Position is still short (-5) → direction mismatch → TP/SL NOT applied.
        let pos = ts.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(-5));
        assert!(pos.conditional_order_above.is_none());
        assert!(pos.conditional_order_below.is_none());
    }

    /// Maker's resting bid (buy) has TP/SL, but maker's position flipped to
    /// short before this fill. The bid partially closes the short but doesn't
    /// flip it → direction mismatch → TP/SL dropped on maker's position.
    #[test]
    fn co11_maker_child_order_dropped_on_direction_mismatch() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);

        // Maker_A has a short position of -10 (was long, got flipped).
        let maker_a_state = UserState {
            margin: LARGE_COLLATERAL,
            open_order_count: 1,
            reserved_margin: UsdValue::new_int(12_500),
            positions: {
                let mut p = BTreeMap::new();
                p.insert(pair_id(), Position {
                    size: Quantity::new_int(-10),
                    entry_price: UsdPrice::new_int(50_000),
                    entry_funding_per_unit: FundingPerUnit::ZERO,
                    conditional_order_above: None,
                    conditional_order_below: None,
                });
                p
            },
            ..Default::default()
        };
        USER_STATES
            .save(&mut ctx.storage, MAKER_A, &maker_a_state)
            .unwrap();

        // Maker_A's resting bid (buy 5) with TP/SL intended for a long.
        let inverted_price = !UsdPrice::new_int(50_000);
        let key = (pair_id(), inverted_price, Uint64::new(100));
        let order = LimitOrder {
            user: MAKER_A,
            size: Quantity::new_int(5),
            reduce_only: false,
            reserved_margin: UsdValue::new_int(12_500),
            created_at: Timestamp::from_nanos(0),
            tp: make_tp(55_000),
            sl: make_sl(45_000),
        };
        BIDS.save(&mut ctx.storage, key, &order).unwrap();

        // Taker sells 5 → fills Maker_A's bid.
        let ts = taker_state(&ctx.storage);

        let SubmitOrderOutcome {
            pair_state: _,
            taker_state: _,
            maker_states,
            ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut test_oracle_querier(),
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState {
                short_oi: Quantity::new_int(10),
                ..Default::default()
            },
            &ts,
            UsdPrice::new_int(50_000),
            Quantity::new_int(-5), // sell
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            None,
            &mut EventBuilder::new(),
        )
        .unwrap();

        // Maker_A bought 5, reducing short from -10 to -5. Still short →
        // direction mismatch → TP/SL NOT applied.
        let maker_state = maker_states.get(&MAKER_A).unwrap();
        let pos = maker_state.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.size, Quantity::new_int(-5));
        assert!(pos.conditional_order_above.is_none());
        assert!(pos.conditional_order_below.is_none());
    }

    /// Fill price is below the SL trigger price — the SL condition is already
    /// met at fill time. The SL is still applied; the cron will trigger it on
    /// the next tick.
    ///
    /// This is correct: the user said "close if price < $49k", and the fill
    /// happened at $48k which is already below $49k. Applying the SL (and
    /// letting the cron close it) is consistent with the user's stated intent.
    ///
    /// Wrong behavior: silently dropping the SL because the condition is
    /// already met — this would remove protection the user explicitly
    /// requested.
    #[test]
    fn co12_child_order_applied_even_when_fill_price_below_sl() {
        let mut ctx = MockContext::new()
            .with_sender(TAKER)
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        setup_storage(&mut ctx.storage);
        setup_taker(&mut ctx.storage, LARGE_COLLATERAL);

        // Resting ask at $48k — the fill will happen at this price.
        place_ask(&mut ctx.storage, MAKER_A, 48_000, 10, 100);

        let ts = taker_state(&ctx.storage);

        // Oracle at $48k (matches the fill price).
        let mut oracle = OracleQuerier::new_mock(hash_map! {
            pair_id() => PrecisionedPrice::new(
                Udec128::new_percent(4_800_000), // $48,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        // Market buy 10 with SL @ $49k. The fill price ($48k) is below the
        // SL trigger ($49k), so the SL condition is already met.
        let SubmitOrderOutcome {
            taker_state: ts, ..
        } = _submit_order(
            &ctx.storage,
            TAKER,
            CONTRACT,
            Timestamp::from_seconds(0),
            &mut oracle,
            &test_param(),
            &State::default(),
            &pair_id(),
            &test_pair_param(),
            &PairState::default(),
            &ts,
            UsdPrice::new_int(48_000),
            Quantity::new_int(10),
            OrderKind::Market {
                max_slippage: Dimensionless::new_permille(100),
            },
            false,
            None,
            make_sl(49_000),
            &mut EventBuilder::new(),
        )
        .unwrap();

        let pos = ts.positions.get(&pair_id()).unwrap();
        assert_eq!(pos.entry_price, UsdPrice::new_int(48_000));

        // SL is applied even though the trigger condition is already met.
        let below = pos.conditional_order_below.as_ref().unwrap();
        assert_eq!(below.trigger_price, UsdPrice::new_int(49_000));
    }

    /// Helper: load taker state (returns default if missing).
    fn taker_state(storage: &dyn Storage) -> UserState {
        USER_STATES
            .may_load(storage, TAKER)
            .unwrap()
            .unwrap_or_default()
    }

    fn setup_taker(storage: &mut dyn Storage, margin: UsdValue) {
        let ts = UserState {
            margin,
            ..Default::default()
        };
        USER_STATES.save(storage, TAKER, &ts).unwrap();
    }
}
