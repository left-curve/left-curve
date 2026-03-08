use {
    crate::{
        ASKS, BIDS, NoCachePerpQuerier, PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES,
        core::{
            compute_bankruptcy_price, compute_close_schedule, compute_maintenance_margin,
            compute_user_equity, is_liquidatable,
        },
        execute::{
            cancel_order::_cancel_all_orders,
            oracle,
            submit_order::{match_order, settle_fill, settle_pnls},
        },
        flush_volumes,
        liquidity_depth::{decrease_liquidity_depths, increase_liquidity_depths},
        position_index::{
            PositionIndexUpdate, apply_position_index_updates, compute_position_diff,
        },
        price::may_invert_price,
        state::{LONGS, SHORTS},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        perps::{
            BadDebtCovered, Deleveraged, Liquidated, Order, OrderId, PairId, PairParam, PairState,
            Param, ReasonForOrderRemoval, State, UserState,
        },
    },
    grug::{
        Addr, EventBuilder, MutableCtx, NumberConst, Order as IterationOrder, Response, Storage,
        Timestamp,
    },
    std::collections::{BTreeMap, btree_map::Entry},
};

/// Liquidate an underwater trader by closing their positions.
///
/// Unfilled positions are ADL'd against counter-parties at the bankruptcy price.
/// Any remaining bad debt is absorbed by the insurance fund.
///
/// Mutates: `STATE`, `PAIR_STATES`, `USER_STATES` (liquidated user + makers +
/// ADL counter-parties), `LONGS`, `SHORTS`.
///
/// Returns: empty `Response` (all PnL/fees settled via internal margins).
pub fn liquidate(ctx: MutableCtx, user: Addr) -> anyhow::Result<Response> {
    // --------------------- 1. Preparation + basic checks ---------------------

    ensure!(user != ctx.contract, "cannot liquidate the vault");

    let param = PARAM.load(ctx.storage)?;
    let mut state = STATE.load(ctx.storage)?;

    let mut user_state = USER_STATES.may_load(ctx.storage, user)?.unwrap_or_default();

    let mut oracle_querier = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier);

    let mut events = EventBuilder::new();

    // -------------------- 2. Cancel all resting orders -----------------------

    _cancel_all_orders(
        ctx.storage,
        user,
        &mut user_state,
        Some(&mut events),
        ReasonForOrderRemoval::Liquidated,
    )?;

    // ------------------- 3. Load pair params and states ---------------------

    // Collect pair IDs first to avoid borrow conflicts.
    let pair_ids = user_state.positions.keys().cloned().collect::<Vec<_>>();

    let mut pair_params = BTreeMap::new();
    let mut pair_states = BTreeMap::new();

    for pair_id in &pair_ids {
        let pair_param = PAIR_PARAMS.load(ctx.storage, pair_id)?;
        let pair_state = PAIR_STATES.load(ctx.storage, pair_id)?;

        pair_params.insert(pair_id.clone(), pair_param);
        pair_states.insert(pair_id.clone(), pair_state);
    }

    // -------------------- 4. Compute oracle prices ---------------------------

    let mut oracle_prices = BTreeMap::new();

    for pair_id in &pair_ids {
        oracle_prices.insert(
            pair_id.clone(),
            oracle_querier.query_price_for_perps(pair_id)?,
        );
    }

    // --------------------------- 5. Business logic ---------------------------

    let (maker_states, order_mutations, index_updates, volumes) = _liquidate(
        ctx.storage,
        user,
        ctx.contract,
        ctx.block.timestamp,
        &mut oracle_querier,
        &param,
        &mut state,
        &pair_params,
        &mut pair_states,
        &mut user_state,
        &oracle_prices,
        &mut events,
    )?;

    // --------------------- 6. Apply state changes ----------------------------

    flush_volumes(ctx.storage, ctx.block.timestamp, &volumes)?;

    STATE.save(ctx.storage, &state)?;

    for (pair_id, pair_state) in &pair_states {
        PAIR_STATES.save(ctx.storage, pair_id, pair_state)?;
    }

    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, user)?;
    } else {
        USER_STATES.save(ctx.storage, user, &user_state)?;
    }

    for (addr, maker_state) in &maker_states {
        if maker_state.is_empty() {
            USER_STATES.remove(ctx.storage, *addr)?;
        } else {
            USER_STATES.save(ctx.storage, *addr, maker_state)?;
        }
    }

    // -------------------- 7. Apply order mutations ---------------------------

    for (pair_id, taker_is_bid, stored_price, order_id, mutation, pre_fill_abs_size) in
        order_mutations
    {
        let order_key = (pair_id.clone(), stored_price, order_id);

        let maker_book = if taker_is_bid {
            ASKS
        } else {
            BIDS
        };

        // The maker is on the opposite side of the taker.
        let maker_is_bid = !taker_is_bid;
        let real_price = may_invert_price(stored_price, maker_is_bid);

        let pair_param = pair_params.get(&pair_id).unwrap();

        // Complete remove the order's liquidity depth contribution, and re-add
        // the remaining size (if any) to prevent notional drift.
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

    // -------------------- 8. Apply position index updates --------------------

    apply_position_index_updates(ctx.storage, &index_updates)?;

    Ok(Response::new().add_events(events)?)
}

/// Mutates:
///
/// - `state.insurance_fund` — receives liquidation fee, covers bad debt.
/// - `pair_states` — OI updated per fill.
/// - `user_state.positions` — closed (partially or fully) per the schedule.
/// - `user_state.margin` — adjusted by settled PnLs, fees, and bad debt.
///
/// Returns:
///
/// - Maker `UserState`s to persist (includes any book makers and ADL counter-parties).
/// - Order mutations to apply.
/// - Position index updates to apply.
fn _liquidate(
    storage: &dyn Storage,
    user: Addr,
    contract: Addr,
    current_time: Timestamp,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    state: &mut State,
    pair_params: &BTreeMap<PairId, PairParam>,
    pair_states: &mut BTreeMap<PairId, PairState>,
    user_state: &mut UserState,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    events: &mut EventBuilder,
) -> anyhow::Result<(
    BTreeMap<Addr, UserState>,
    Vec<(PairId, bool, UsdPrice, OrderId, Option<Order>, Quantity)>,
    Vec<PositionIndexUpdate>,
    BTreeMap<Addr, UsdValue>,
)> {
    // -------------------- Step 1: Assert liquidatable -------------------------

    let perp_querier = NoCachePerpQuerier::new_local(storage);

    ensure!(
        is_liquidatable(oracle_querier, &perp_querier, user_state)?,
        "user is not liquidatable"
    );

    // ------------- Step 2: Compute close schedule (largest-MM-first) ----------

    let equity = compute_user_equity(oracle_querier, &perp_querier, user_state)?;
    let total_mm = compute_maintenance_margin(oracle_querier, &perp_querier, user_state)?;
    let deficit = total_mm.checked_sub(equity)?;

    let schedule = compute_close_schedule(user_state, pair_params, oracle_prices, deficit)?;

    // -------- Step 3: Execute closes via the order book + ADL ----------------

    let mut all_maker_states = BTreeMap::new();

    let (
        all_pnls,
        mut all_fees,
        all_order_mutations,
        closed_notional,
        all_index_updates,
        all_volumes,
    ) = execute_close_schedule(
        storage,
        user,
        contract,
        current_time,
        param,
        pair_states,
        user_state,
        &mut all_maker_states,
        &schedule,
        oracle_prices,
        events,
    )?;

    // -------------------- Step 4: Liquidation fee → insurance fund -----------

    let liq_fee = compute_liquidation_fee(
        &all_pnls,
        user,
        closed_notional,
        param.liquidation_fee_rate,
        user_state.margin,
    )?;

    if liq_fee.is_non_zero() {
        all_fees
            .entry(user)
            .or_default()
            .checked_add_assign(liq_fee)?;
    }

    // ----------------------- Step 5: Settle PnLs ------------------------------

    // Ensure the vault's UserState is in the map for fee settlement.
    all_maker_states.entry(contract).or_insert_with(|| {
        USER_STATES
            .may_load(storage, contract)
            .unwrap()
            .unwrap_or_default()
    });

    settle_pnls(
        contract,
        param,
        state,
        user,
        user_state,
        &mut all_maker_states,
        all_pnls,
        all_fees,
    )?;

    // Route liquidation fee to insurance fund (not vault margin).
    // settle_pnls added the fee to the vault's margin and subtracted from user.
    // Reverse the vault credit and add to insurance fund instead.
    if liq_fee.is_non_zero() {
        all_maker_states
            .get_mut(&contract)
            .unwrap()
            .margin
            .checked_sub_assign(liq_fee)?;
        state.insurance_fund.checked_add_assign(liq_fee)?;
    }

    // -------------------- Step 6: Bad debt → insurance fund ------------------

    if user_state.margin.is_negative() {
        let bad_debt = user_state.margin.checked_abs()?;
        user_state.margin = UsdValue::ZERO;

        // Deduct from insurance fund (can go negative as last resort).
        state.insurance_fund.checked_sub_assign(bad_debt)?;

        events.push(BadDebtCovered {
            liquidated_user: user,
            amount: bad_debt,
            insurance_fund_remaining: state.insurance_fund,
        })?;
    }

    Ok((
        all_maker_states,
        all_order_mutations,
        all_index_updates,
        all_volumes,
    ))
}

/// Execute the close schedule against the order book, with ADL for any unfilled
/// remainder.
///
/// `maker_states` is a shared map of maker `UserState`s that persists across
/// `match_order` calls.
fn execute_close_schedule(
    storage: &dyn Storage,
    user: Addr,
    contract: Addr,
    current_time: Timestamp,
    param: &Param,
    pair_states: &mut BTreeMap<PairId, PairState>,
    user_state: &mut UserState,
    maker_states: &mut BTreeMap<Addr, UserState>,
    schedule: &[(PairId, Quantity)],
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    events: &mut EventBuilder,
) -> anyhow::Result<(
    BTreeMap<Addr, UsdValue>,
    BTreeMap<Addr, UsdValue>,
    Vec<(PairId, bool, UsdPrice, OrderId, Option<Order>, Quantity)>,
    UsdValue,
    Vec<PositionIndexUpdate>,
    BTreeMap<Addr, UsdValue>,
)> {
    // Zero-fee param for liquidation fills.
    let liq_param = Param {
        base_taker_fee_rate: Dimensionless::ZERO,
        base_maker_fee_rate: Dimensionless::ZERO,
        tiered_taker_fee_rate: BTreeMap::new(),
        tiered_maker_fee_rate: BTreeMap::new(),
        ..param.clone()
    };

    let mut all_pnls = BTreeMap::<_, UsdValue>::new();
    let mut all_fees = BTreeMap::<_, UsdValue>::new();
    let mut all_volumes = BTreeMap::<_, UsdValue>::new();
    let mut all_order_mutations = Vec::new();
    let mut closed_notional = UsdValue::ZERO;
    let mut all_index_updates = Vec::new();

    for (pair_id, close_size) in schedule {
        let pair_state = pair_states.get_mut(pair_id).unwrap();
        let oracle_price = oracle_prices[pair_id];

        let taker_is_bid = close_size.is_positive();
        let target_price = if taker_is_bid {
            UsdPrice::MAX
        } else {
            UsdPrice::ZERO
        };

        let (unfilled, pnls, fees, volumes, order_mutations, index_updates) = match_order(
            storage,
            user,
            contract,
            current_time,
            &liq_param,
            pair_id,
            pair_state,
            user_state,
            taker_is_bid,
            OrderId::ZERO,
            maker_states,
            target_price,
            *close_size,
            events,
        )?;

        // Merge PnLs.
        for (addr, pnl) in pnls {
            all_pnls.entry(addr).or_default().checked_add_assign(pnl)?;
        }

        // Merge fees.
        for (addr, fee) in fees {
            all_fees.entry(addr).or_default().checked_add_assign(fee)?;
        }

        // Merge volumes.
        for (addr, vol) in volumes {
            all_volumes
                .entry(addr)
                .or_default()
                .checked_add_assign(vol)?;
        }

        // Collect order mutations with pair context.
        for (stored_price, order_id, mutation, pre_fill_abs_size) in order_mutations {
            all_order_mutations.push((
                pair_id.clone(),
                taker_is_bid,
                stored_price,
                order_id,
                mutation,
                pre_fill_abs_size,
            ));
        }

        all_index_updates.extend(index_updates);

        // Track closed notional for fee calculation.
        let filled = close_size.checked_sub(unfilled)?;
        closed_notional.checked_add_assign(filled.checked_abs()?.checked_mul(oracle_price)?)?;

        // ADL: if there is unfilled remainder, ADL against counter-positions
        // at the bankruptcy price.
        if unfilled.is_non_zero() {
            // Snapshot the user's accumulated PnL/fees before passing mutable refs.
            let user_pnl_snapshot = all_pnls.get(&user).copied().unwrap_or(UsdValue::ZERO);
            let user_fee_snapshot = all_fees.get(&user).copied().unwrap_or(UsdValue::ZERO);

            let (adl_size, adl_price) = execute_adl(
                storage,
                user,
                pair_id,
                pair_state,
                user_state,
                maker_states,
                unfilled,
                oracle_prices,
                user_pnl_snapshot,
                user_fee_snapshot,
                &mut all_pnls,
                &mut all_fees,
                &mut all_volumes,
                &mut all_index_updates,
                events,
            )?;

            // Add ADL notional.
            closed_notional
                .checked_add_assign(adl_size.checked_abs()?.checked_mul(oracle_price)?)?;

            events.push(Liquidated {
                user,
                pair_id: pair_id.clone(),
                adl_size,
                adl_price: Some(adl_price),
            })?;
        } else {
            events.push(Liquidated {
                user,
                pair_id: pair_id.clone(),
                adl_size: Quantity::ZERO,
                adl_price: None,
            })?;
        }
    }

    Ok((
        all_pnls,
        all_fees,
        all_order_mutations,
        closed_notional,
        all_index_updates,
        all_volumes,
    ))
}

/// ADL the unfilled remainder of a liquidation against counter-positions.
///
/// Returns: (total ADL size, bankruptcy price used).
fn execute_adl(
    storage: &dyn Storage,
    user: Addr,
    pair_id: &PairId,
    pair_state: &mut PairState,
    user_state: &mut UserState,
    maker_states: &mut BTreeMap<Addr, UserState>,
    unfilled: Quantity,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    // Snapshot of the user's accumulated PnL/fees before this ADL round.
    user_pnl_snapshot: UsdValue,
    user_fee_snapshot: UsdValue,
    // Mutable PnL/fee/volume maps to accumulate ADL results.
    all_pnls: &mut BTreeMap<Addr, UsdValue>,
    all_fees: &mut BTreeMap<Addr, UsdValue>,
    all_volumes: &mut BTreeMap<Addr, UsdValue>,
    index_updates: &mut Vec<PositionIndexUpdate>,
    events: &mut EventBuilder,
) -> anyhow::Result<(Quantity, UsdPrice)> {
    let bankruptcy_price = compute_bankruptcy_price(
        user_state,
        pair_id,
        unfilled.checked_abs()?,
        oracle_prices,
        user_pnl_snapshot,
        user_fee_snapshot,
    )?;

    let mut remaining = unfilled;
    let taker_is_selling = unfilled.is_negative();

    // Collect counter-parties to avoid borrow conflicts during iteration.
    let counter_parties: Vec<(UsdPrice, Addr)> = if taker_is_selling {
        // Need shorts (buyers). Most profitable shorts have highest entry price.
        SHORTS
            .prefix(pair_id.clone())
            .keys(storage, None, None, IterationOrder::Descending)
            .collect::<Result<_, _>>()?
    } else {
        // Need longs (sellers). Most profitable longs have lowest entry price.
        LONGS
            .prefix(pair_id.clone())
            .keys(storage, None, None, IterationOrder::Ascending)
            .collect::<Result<_, _>>()?
    };

    let mut total_adl_size = Quantity::ZERO;

    for (entry_price, counter_user) in counter_parties {
        if remaining.is_zero() {
            break;
        }

        // Skip the user being liquidated (they can't be their own counter-party).
        if counter_user == user {
            continue;
        }

        // Load counter-party state.
        let counter_state = match maker_states.entry(counter_user) {
            Entry::Vacant(e) => {
                let maybe_state = USER_STATES.may_load(storage, counter_user)?;
                e.insert(maybe_state.unwrap_or_default())
            },
            Entry::Occupied(e) => e.into_mut(),
        };

        // Verify counter-party still has this position (may have been modified
        // by earlier book matching in a shared maker_states map).
        let counter_position = match counter_state.positions.get(pair_id) {
            Some(pos) if pos.entry_price == entry_price => pos,
            _ => continue,
        };

        let user_close = {
            let counter_size = counter_position.size.checked_abs()?;
            let fill_amount = remaining.checked_abs()?.min(counter_size);
            if taker_is_selling {
                fill_amount.checked_neg()?
            } else {
                fill_amount
            }
        };

        // User side:
        let old_user_pos = user_state.positions.get(pair_id).cloned();
        settle_fill(
            pair_id,
            pair_state,
            user_state,
            user,
            user_close,
            bankruptcy_price,
            Dimensionless::ZERO,
            all_pnls,
            all_fees,
            all_volumes,
            None,
        )?;
        if let Some(diff) = compute_position_diff(
            pair_id,
            user,
            old_user_pos.as_ref(),
            user_state.positions.get(pair_id),
        ) {
            index_updates.push(diff);
        }

        // Counter-party side:
        let old_counter_pos = counter_state.positions.get(pair_id).cloned();
        let counter_pnl = settle_fill(
            pair_id,
            pair_state,
            counter_state,
            counter_user,
            user_close.checked_neg()?,
            bankruptcy_price,
            Dimensionless::ZERO,
            all_pnls,
            all_fees,
            all_volumes,
            None,
        )?;
        if let Some(diff) = compute_position_diff(
            pair_id,
            counter_user,
            old_counter_pos.as_ref(),
            counter_state.positions.get(pair_id),
        ) {
            index_updates.push(diff);
        }

        // Emit Deleveraged event for counter-party.
        events.push(Deleveraged {
            user: counter_user,
            pair_id: pair_id.clone(),
            closing_size: user_close.checked_neg()?,
            fill_price: bankruptcy_price,
            realized_pnl: counter_pnl,
        })?;

        remaining = remaining.checked_sub(user_close)?;
        total_adl_size.checked_add_assign(user_close)?;
    }

    Ok((total_adl_size, bankruptcy_price))
}

/// Compute the liquidation fee, capped at the user's remaining margin after
/// position PnL.
fn compute_liquidation_fee(
    pnls: &BTreeMap<Addr, UsdValue>,
    user: Addr,
    closed_notional: UsdValue,
    liquidation_fee_rate: Dimensionless,
    user_margin: UsdValue,
) -> anyhow::Result<UsdValue> {
    let fee_usd = closed_notional.checked_mul(liquidation_fee_rate)?;
    let user_pnl = pnls.get(&user).copied().unwrap_or(UsdValue::ZERO);
    let remaining_margin = user_margin.checked_add(user_pnl)?.max(UsdValue::ZERO);
    Ok(fee_usd.min(remaining_margin))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES,
            state::{LONGS, OrderKey, SHORTS},
        },
        dango_types::{
            Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue,
            perps::{Order, PairParam, PairState, Param, Position, State, UserState},
        },
        grug::{Addr, Coins, MockContext, Storage, Timestamp, Uint64},
        std::collections::BTreeMap,
    };

    const USER: Addr = Addr::mock(1);
    const MAKER: Addr = Addr::mock(2);
    const COUNTER: Addr = Addr::mock(3);
    const CONTRACT: Addr = Addr::mock(0);

    fn pair_btc() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn default_param() -> Param {
        Param {
            base_taker_fee_rate: Dimensionless::new_permille(10), // 1%
            base_maker_fee_rate: Dimensionless::new_permille(10), // 1%
            liquidation_fee_rate: Dimensionless::new_permille(10), // 1%
            max_open_orders: 100,
            ..Default::default()
        }
    }

    fn btc_pair_param() -> PairParam {
        PairParam {
            initial_margin_ratio: Dimensionless::new_permille(100), // 10%
            maintenance_margin_ratio: Dimensionless::new_permille(50), // 5%
            max_abs_oi: Quantity::new_int(1_000_000),
            ..Default::default()
        }
    }

    /// Set up the contract storage with pair params, pair states, and global params.
    fn setup_storage(
        storage: &mut dyn Storage,
        param: &Param,
        pairs: &[(PairId, PairParam, PairState)],
    ) {
        PARAM.save(storage, param).unwrap();
        STATE.save(storage, &State::default()).unwrap();

        for (pair_id, pair_param, pair_state) in pairs {
            PAIR_PARAMS.save(storage, pair_id, pair_param).unwrap();
            PAIR_STATES.save(storage, pair_id, pair_state).unwrap();
        }
    }

    /// Save a user position and add to LONGS/SHORTS index.
    fn save_position(
        storage: &mut dyn Storage,
        user: Addr,
        pair_id: &PairId,
        size: i128,
        entry_price: i128,
    ) {
        let mut user_state = USER_STATES
            .may_load(storage, user)
            .unwrap()
            .unwrap_or_default();

        user_state.positions.insert(pair_id.clone(), Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(entry_price),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });

        USER_STATES.save(storage, user, &user_state).unwrap();

        // Update position index.
        let ep = UsdPrice::new_int(entry_price);
        if size > 0 {
            LONGS.insert(storage, (pair_id.clone(), ep, user)).unwrap();
        } else {
            SHORTS.insert(storage, (pair_id.clone(), ep, user)).unwrap();
        }
    }

    /// Save a bid order into the book (buy side).
    /// Bids are stored with inverted price so ascending iteration gives best bid first.
    fn save_bid(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        order_id: u64,
        maker: Addr,
        size: i128,
        price: i128,
    ) {
        use crate::price::may_invert_price;
        let stored_price = may_invert_price(UsdPrice::new_int(price), true);
        let key: OrderKey = (pair_id.clone(), stored_price, Uint64::new(order_id));
        let order = Order {
            user: maker,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
        };
        BIDS.save(storage, key, &order).unwrap();
    }

    /// Save an ask order into the book (sell side).
    fn save_ask(
        storage: &mut dyn Storage,
        pair_id: &PairId,
        order_id: u64,
        maker: Addr,
        size: i128,
        price: i128,
    ) {
        let stored_price = UsdPrice::new_int(price);
        let key: OrderKey = (pair_id.clone(), stored_price, Uint64::new(order_id));
        let order = Order {
            user: maker,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
        };
        ASKS.save(storage, key, &order).unwrap();
    }

    fn mock_oracle_querier(pairs: Vec<(PairId, i128)>) -> OracleQuerier<'static> {
        use {dango_types::oracle::PrecisionedPrice, grug::Udec128};
        let mut map = std::collections::HashMap::new();
        for (pair_id, price) in pairs {
            map.insert(
                pair_id,
                PrecisionedPrice::new(
                    Udec128::new_percent(price as u128 * 100),
                    Timestamp::from_seconds(0),
                    8,
                ),
            );
        }
        OracleQuerier::new_mock(map)
    }

    // ======================== Tests ========================

    #[test]
    fn not_liquidatable_errors() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let pair_state = PairState::default();

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state,
        )]);

        // User has long 1 BTC at 50000, oracle at 50000.
        // Collateral = 10000 USD (well above MM = 50000 * 5% = 2500).
        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), PairState::default());

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(50_000));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        user_state.margin = UsdValue::new_int(10_000);

        let mut oracle_querier = mock_oracle_querier(vec![(pair_btc(), 50_000)]);
        let mut state = STATE.load(&ctx.storage).unwrap();

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oracle_querier,
            &param,
            &mut state,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut EventBuilder::new(),
        );

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("not liquidatable"),
            "expected 'not liquidatable' error"
        );
    }

    #[test]
    fn vault_self_liquidation_rejected() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        let param = default_param();
        let pair_state = PairState::default();

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state,
        )]);

        let result = liquidate(ctx.as_mutable(), CONTRACT);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot liquidate the vault"),
            "expected 'cannot liquidate the vault' error"
        );
    }

    #[test]
    fn single_position_full_close_on_book() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let pair_state = PairState {
            long_oi: Quantity::new_int(10),
            ..Default::default()
        };

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // User has long 10 BTC at entry 50000. Oracle is now 47500.
        // equity < MM → liquidatable
        save_position(&mut ctx.storage, USER, &pair_btc(), 10, 50_000);

        // Maker with bids to absorb the liquidation (user is selling a long).
        let maker_state = UserState {
            margin: UsdValue::new_int(100_000),
            open_order_count: 1,
            ..Default::default()
        };
        USER_STATES
            .save(&mut ctx.storage, MAKER, &maker_state)
            .unwrap();

        save_bid(&mut ctx.storage, &pair_btc(), 1, MAKER, 10, 47_500);

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(47_500));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        user_state.margin = UsdValue::new_int(2_400);

        let mut oracle_querier = mock_oracle_querier(vec![(pair_btc(), 47_500)]);
        let mut state = STATE.load(&ctx.storage).unwrap();

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oracle_querier,
            &param,
            &mut state,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut EventBuilder::new(),
        );

        assert!(result.is_ok(), "liquidation failed: {:?}", result.err());

        // User's position should be closed.
        assert!(
            user_state.positions.is_empty(),
            "user should have no positions after liquidation"
        );
    }

    #[test]
    fn single_position_full_adl_empty_book() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let pair_state = PairState {
            long_oi: Quantity::new_int(10),
            short_oi: Quantity::new_int(10),
            ..Default::default()
        };

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // User long 10 BTC at 50000, oracle 47500 → liquidatable.
        save_position(&mut ctx.storage, USER, &pair_btc(), 10, 50_000);

        // Counter-party has short 10 BTC at $55000 (profitable).
        save_position(&mut ctx.storage, COUNTER, &pair_btc(), -10, 55_000);
        let mut counter_state = USER_STATES.load(&ctx.storage, COUNTER).unwrap();
        counter_state.margin = UsdValue::new_int(100_000);
        USER_STATES
            .save(&mut ctx.storage, COUNTER, &counter_state)
            .unwrap();

        // No book orders → ADL should happen.

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(47_500));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        user_state.margin = UsdValue::new_int(2_400);

        let mut oracle_querier = mock_oracle_querier(vec![(pair_btc(), 47_500)]);
        let mut state = STATE.load(&ctx.storage).unwrap();

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oracle_querier,
            &param,
            &mut state,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut EventBuilder::new(),
        );

        assert!(result.is_ok(), "ADL failed: {:?}", result.err());

        let (maker_states, ..) = result.unwrap();

        // User's position should be closed.
        assert!(user_state.positions.is_empty());

        // Counter-party's position should be reduced/closed.
        let counter_final = &maker_states[&COUNTER];
        assert!(
            !counter_final.positions.contains_key(&pair_btc())
                || counter_final.positions[&pair_btc()]
                    .size
                    .checked_abs()
                    .unwrap()
                    < Quantity::new_int(10),
            "counter-party position should be reduced"
        );
    }

    #[test]
    fn liquidation_fee_to_insurance_fund() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let pair_state = PairState {
            long_oi: Quantity::new_int(1),
            ..Default::default()
        };

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // User long 1 BTC @ $50,000, margin $2,500, oracle $48,000.
        // Equity = $2,500 + ($48,000-$50,000) = $500.
        // MM = $48,000 * 5% = $2,400. Equity < MM → liquidatable.
        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);

        // Bid on book at $49,000 (better than bp=$47,500).
        // Fill at $49,000 leaves margin for the liq fee.
        let maker_state = UserState {
            margin: UsdValue::new_int(100_000),
            open_order_count: 1,
            ..Default::default()
        };
        USER_STATES
            .save(&mut ctx.storage, MAKER, &maker_state)
            .unwrap();
        save_bid(&mut ctx.storage, &pair_btc(), 1, MAKER, 1, 49_000);

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(48_000));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        user_state.margin = UsdValue::new_int(2_500);

        let mut oracle_querier = mock_oracle_querier(vec![(pair_btc(), 48_000)]);
        let mut state = STATE.load(&ctx.storage).unwrap();

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oracle_querier,
            &param,
            &mut state,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut EventBuilder::new(),
        );

        assert!(result.is_ok(), "liq fee test failed: {:?}", result.err());

        // Fill at $49,000: PnL = 1*($49,000-$50,000) = -$1,000.
        // Remaining margin = $2,500 - $1,000 = $1,500.
        // Liq fee = min($49,000 * 1%, $1,500) = min($490, $1,500) = $490.
        // Insurance fund should have received the fee.
        assert!(
            state.insurance_fund > UsdValue::ZERO,
            "insurance fund should have received the liquidation fee"
        );
    }

    #[test]
    fn bad_debt_covered_by_insurance_fund() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = Param {
            liquidation_fee_rate: Dimensionless::ZERO,
            ..default_param()
        };
        let pair_state = PairState {
            long_oi: Quantity::new_int(10),
            ..Default::default()
        };

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // User long 10 BTC @ $50,000, margin $2,400, oracle $47,500.
        // PnL = 10*(47500-50000) = -$25,000. Equity = $2,400 - $25,000 = -$22,600.
        save_position(&mut ctx.storage, USER, &pair_btc(), 10, 50_000);

        // Bid on book at oracle price from maker. When the liquidation taker
        // sells into this bid, the fill realizes the full unrealized loss,
        // driving margin negative → bad debt.
        let maker_state = UserState {
            margin: UsdValue::new_int(100_000),
            open_order_count: 1,
            ..Default::default()
        };
        USER_STATES
            .save(&mut ctx.storage, MAKER, &maker_state)
            .unwrap();
        save_bid(&mut ctx.storage, &pair_btc(), 1, MAKER, 10, 47_500);

        // Seed insurance fund with $500.
        let mut state = STATE.load(&ctx.storage).unwrap();
        state.insurance_fund = UsdValue::new_int(500);
        STATE.save(&mut ctx.storage, &state).unwrap();

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(47_500));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        user_state.margin = UsdValue::new_int(2_400);

        let mut oracle_querier = mock_oracle_querier(vec![(pair_btc(), 47_500)]);

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oracle_querier,
            &param,
            &mut state,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut EventBuilder::new(),
        );

        assert!(result.is_ok(), "bad debt test failed: {:?}", result.err());

        // User margin should be floored at zero.
        assert_eq!(user_state.margin, UsdValue::ZERO);

        // Fill at $47,500: PnL = 10*($47,500-$50,000) = -$25,000.
        // Margin = $2,400 - $25,000 = -$22,600. Bad debt = $22,600.
        // Insurance fund = $500 - $22,600 = -$22,100.
        assert!(
            state.insurance_fund < UsdValue::new_int(500),
            "insurance fund should have been reduced by bad debt"
        );
    }

    /// Proves the cancel-before-match invariant: when a user is liquidated,
    /// their resting orders are removed *before* the close schedule matches
    /// against the book, so self-trades cannot occur.
    #[test]
    fn liquidation_cancels_user_orders_before_matching() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let pair_state = PairState {
            long_oi: Quantity::new_int(1),
            ..Default::default()
        };

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // USER has long 1 BTC at $50,000. Oracle at $47,500 → deeply underwater.
        // Equity = $100 + ($47,500-$50,000) = -$2,400.
        // MM = $47,500 * 5% = $2,375. Equity < MM → liquidatable.
        // Deficit = $2,375 - (-$2,400) = $4,775 → full close (close_amount ≥ 1).
        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);

        // USER also has a resting ask (sell) on the book — this must be
        // cancelled before matching, otherwise it could self-match.
        save_ask(&mut ctx.storage, &pair_btc(), 10, USER, -1, 48_000);

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        user_state.margin = UsdValue::new_int(100);
        user_state.open_order_count = 1;
        USER_STATES
            .save(&mut ctx.storage, USER, &user_state)
            .unwrap();

        // MAKER has a bid to absorb the liquidation close (user sells long).
        let maker_state = UserState {
            margin: UsdValue::new_int(100_000),
            open_order_count: 1,
            ..Default::default()
        };
        USER_STATES
            .save(&mut ctx.storage, MAKER, &maker_state)
            .unwrap();
        save_bid(&mut ctx.storage, &pair_btc(), 1, MAKER, 1, 47_500);

        // Step 1: Cancel all user orders (mirrors the public `liquidate` flow).
        let mut events = EventBuilder::new();
        _cancel_all_orders(
            &mut ctx.storage,
            USER,
            &mut user_state,
            Some(&mut events),
            ReasonForOrderRemoval::Liquidated,
        )
        .unwrap();

        // The user's ask should be gone from the book.
        let user_asks: Vec<_> = ASKS
            .idx
            .user
            .prefix(USER)
            .range(&ctx.storage, None, None, IterationOrder::Ascending)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            user_asks.is_empty(),
            "user's ask should have been cancelled"
        );
        assert_eq!(user_state.open_order_count, 0);

        // Step 2: Run _liquidate (same as the public function does next).
        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(47_500));

        let mut oracle_querier = mock_oracle_querier(vec![(pair_btc(), 47_500)]);
        let mut state = STATE.load(&ctx.storage).unwrap();

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            Timestamp::ZERO,
            &mut oracle_querier,
            &param,
            &mut state,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut EventBuilder::new(),
        );

        assert!(result.is_ok(), "liquidation failed: {:?}", result.err());

        let (maker_states, ..) = result.unwrap();

        // User's position should be fully closed against MAKER's bid.
        assert!(
            user_state.positions.is_empty(),
            "user should have no positions after liquidation"
        );

        // MAKER should now hold a position (absorbed the user's long).
        assert!(
            maker_states[&MAKER].positions.contains_key(&pair_btc()),
            "maker should have a position after absorbing liquidation"
        );
    }
}
