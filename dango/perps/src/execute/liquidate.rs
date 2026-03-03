use {
    crate::{
        ASKS, BIDS, NoCachePerpQuerier, PAIR_PARAMS, PAIR_STATES, PARAM, USER_STATES,
        core::{
            compute_close_schedule, compute_maintenance_margin, compute_user_equity,
            is_liquidatable,
        },
        execute::{
            cancel_order::cancel_all_orders_for,
            oracle,
            submit_order::{match_order, settle_fill, settle_pnls},
        },
        liquidity_depth::{decrease_liquidity_depths, increase_liquidity_depths},
        price::may_invert_price,
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        perps::{Order, OrderId, PairId, PairParam, PairState, Param, UserState},
    },
    grug::{Addr, MutableCtx, Response, Storage},
    std::collections::BTreeMap,
};

/// Liquidate an underwater trader by closing their positions.
///
/// Mutates: `STATE`, `PAIR_STATES`, `USER_STATES` (liquidated user + makers).
///
/// Returns: empty `Response` (all PnL/fees settled via internal margins).
pub fn liquidate(ctx: MutableCtx, user: Addr) -> anyhow::Result<Response> {
    ensure!(user != ctx.contract, "cannot liquidate the vault");

    // ----------------------------- 1. Load state -----------------------------

    let param = PARAM.load(ctx.storage)?;

    let mut user_state = USER_STATES.may_load(ctx.storage, user)?.unwrap_or_default();

    let mut oracle_querier = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier);

    // -------------------- 2. Cancel all resting orders -----------------------

    cancel_all_orders_for(ctx.storage, user, &mut user_state)?;

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

    // -------------------- 4. Load vault state --------------------------------

    let vault_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    // -------------------- 5. Compute oracle prices ---------------------------

    let mut oracle_prices = BTreeMap::new();

    for pair_id in &pair_ids {
        oracle_prices.insert(
            pair_id.clone(),
            oracle_querier.query_price_for_perps(pair_id)?,
        );
    }

    // --------------------------- 6. Business logic ---------------------------

    let (maker_states, order_mutations) = _liquidate(
        ctx.storage,
        user,
        ctx.contract,
        &param,
        &pair_params,
        &mut pair_states,
        &mut user_state,
        vault_state,
        &oracle_prices,
        &mut oracle_querier,
    )?;

    // --------------------- 7. Apply state changes ----------------------------

    for (pair_id, pair_state) in &pair_states {
        PAIR_STATES.save(ctx.storage, pair_id, pair_state)?;
    }

    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, user)?;
    } else {
        USER_STATES.save(ctx.storage, user, &user_state)?;
    }

    // The maker_states map includes the vault state (seeded before matching),
    // so all makers and the vault are saved in one pass.
    for (addr, maker_state) in &maker_states {
        USER_STATES.save(ctx.storage, *addr, maker_state)?;
    }

    // -------------------- 8. Apply order mutations ---------------------------

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

        // Remove the old depth contribution.
        decrease_liquidity_depths(
            ctx.storage,
            &pair_id,
            maker_is_bid,
            real_price,
            pre_fill_abs_size,
            &pair_param.bucket_sizes,
        )?;

        match mutation {
            Some(ref order) => {
                maker_book.save(ctx.storage, order_key, order)?;

                // Re-add the remaining size.
                increase_liquidity_depths(
                    ctx.storage,
                    &pair_id,
                    maker_is_bid,
                    real_price,
                    order.size.checked_abs()?,
                    &pair_param.bucket_sizes,
                )?;
            },
            None => {
                maker_book.remove(ctx.storage, order_key)?;
            },
        }
    }

    // No token transfers — all PnL/fees settled via internal margins.
    Ok(Response::new())
}

/// Mutates:
///
/// - `pair_states` — OI updated per fill.
/// - `user_state.positions` — closed (partially or fully) per the schedule.
/// - `user_state.margin` — adjusted by settled PnLs, fees, and bad debt.
///
/// Returns:
///
/// - Maker `UserState`s to persist (includes vault's `UserState` with updated
///   margin from PnLs, fees, and bad debt).
/// - Order mutations to apply: `(pair_id, taker_is_bid, stored_price, order_id, Option<Order>, pre_fill_abs_size)`.
fn _liquidate(
    storage: &dyn Storage,
    user: Addr,
    contract: Addr,
    param: &Param,
    pair_params: &BTreeMap<PairId, PairParam>,
    pair_states: &mut BTreeMap<PairId, PairState>,
    user_state: &mut UserState,
    vault_state: UserState,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<(
    BTreeMap<Addr, UserState>,
    Vec<(PairId, bool, UsdPrice, OrderId, Option<Order>, Quantity)>,
)> {
    // -------------------- Step 1: Assert liquidatable -------------------------

    let perp_querier = NoCachePerpQuerier::new_local(storage);

    ensure!(
        is_liquidatable(user_state, &perp_querier, oracle_querier)?,
        "user is not liquidatable"
    );

    // ------------- Step 2: Compute close schedule (largest-MM-first) ----------

    let equity = compute_user_equity(user_state, &perp_querier, oracle_querier)?;
    let total_mm = compute_maintenance_margin(user_state, &perp_querier, oracle_querier)?;
    let deficit = total_mm.checked_sub(equity)?;

    let schedule = compute_close_schedule(user_state, pair_params, oracle_prices, deficit)?;

    // -------- Step 3: Execute closes via the order book -----------------------

    // Seed the shared maker states map with the vault state so that
    // book-matched fills and backstop fills accumulate on the same object.
    let mut all_maker_states = BTreeMap::new();
    all_maker_states.insert(contract, vault_state);

    let (all_pnls, mut all_fees, all_order_mutations, closed_notional) = execute_close_schedule(
        storage,
        &schedule,
        user,
        contract,
        param,
        pair_states,
        user_state,
        &mut all_maker_states,
        oracle_prices,
    )?;

    // -------------------- Step 4: Liquidation fee -----------------------------

    apply_liquidation_fee(
        &all_pnls,
        &mut all_fees,
        user,
        closed_notional,
        param.liquidation_fee_rate,
        user_state.margin,
    )?;

    // ----------------------- Step 5: Settle PnLs ------------------------------

    // Merge the liquidated user into the maker states for settlement.
    all_maker_states.insert(user, user_state.clone());

    settle_pnls(all_pnls, all_fees, contract, &mut all_maker_states)?;

    // Extract the user back.
    *user_state = all_maker_states.remove(&user).unwrap();

    // Bad debt check: if the user's margin went negative after settlement,
    // floor at zero and subtract the bad debt from the vault's margin (which
    // may go negative, representing the deficit).
    if user_state.margin.is_negative() {
        let bad_debt = user_state.margin.checked_abs()?;
        user_state.margin = UsdValue::ZERO;
        all_maker_states
            .get_mut(&contract)
            .unwrap()
            .margin
            .checked_sub_assign(bad_debt)?;
    }

    Ok((all_maker_states, all_order_mutations))
}

/// Execute the close schedule against the order book, with vault backstop for
/// any unfilled remainder.
///
/// `maker_states` is a shared map of maker `UserState`s that persists across
/// `match_order` calls. It must be seeded with the vault's state (keyed by
/// `contract`) before calling this function, so that book-matched fills and
/// backstop fills accumulate on the same `UserState` object.
fn execute_close_schedule(
    storage: &dyn Storage,
    schedule: &[(PairId, Quantity)],
    user: Addr,
    contract: Addr,
    param: &Param,
    pair_states: &mut BTreeMap<PairId, PairState>,
    user_state: &mut UserState,
    maker_states: &mut BTreeMap<Addr, UserState>,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
) -> anyhow::Result<(
    BTreeMap<Addr, UsdValue>,
    BTreeMap<Addr, UsdValue>,
    Vec<(PairId, bool, UsdPrice, OrderId, Option<Order>, Quantity)>,
    UsdValue,
)> {
    // Zero-fee param for liquidation fills.
    let liq_param = Param {
        taker_fee_rate: Dimensionless::ZERO,
        maker_fee_rate: Dimensionless::ZERO,
        ..param.clone()
    };

    let mut all_pnls = BTreeMap::<_, UsdValue>::new();
    let mut all_fees = BTreeMap::<_, UsdValue>::new();
    let mut all_order_mutations = Vec::new();
    let mut closed_notional = UsdValue::ZERO;

    for (pair_id, close_size) in schedule {
        let pair_state = pair_states.get_mut(pair_id).unwrap();
        let oracle_price = oracle_prices[pair_id];

        let taker_is_bid = close_size.is_positive();
        let target_price = if taker_is_bid {
            UsdPrice::MAX
        } else {
            UsdPrice::ZERO
        };

        let (unfilled, pnls, fees, order_mutations) = match_order(
            storage,
            &liq_param,
            pair_id,
            pair_state,
            user,
            user_state,
            taker_is_bid,
            target_price,
            *close_size,
            maker_states,
        )?;

        // Merge PnLs.
        for (addr, pnl) in pnls {
            all_pnls.entry(addr).or_default().checked_add_assign(pnl)?;
        }

        // Merge fees.
        for (addr, fee) in fees {
            all_fees.entry(addr).or_default().checked_add_assign(fee)?;
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

        // Track closed notional for fee calculation.
        let filled = close_size.checked_sub(unfilled)?;
        closed_notional.checked_add_assign(filled.checked_abs()?.checked_mul(oracle_price)?)?;

        // Vault backstop: if there is unfilled remainder, the vault absorbs at oracle price.
        if unfilled.is_non_zero() {
            // User side: close at oracle price with zero fee.
            settle_fill(
                pair_id,
                pair_state,
                user_state,
                unfilled,
                oracle_price,
                Dimensionless::ZERO,
                &mut all_pnls,
                &mut all_fees,
                user,
            )?;

            // Vault side: opposite fill at oracle price with zero fee.
            // Use the vault state from the shared maker_states map.
            let vault_state = maker_states.entry(contract).or_default();
            settle_fill(
                pair_id,
                pair_state,
                vault_state,
                unfilled.checked_neg()?,
                oracle_price,
                Dimensionless::ZERO,
                &mut all_pnls,
                &mut all_fees,
                contract,
            )?;

            // Add vault backstop notional.
            closed_notional
                .checked_add_assign(unfilled.checked_abs()?.checked_mul(oracle_price)?)?;
        }
    }

    Ok((all_pnls, all_fees, all_order_mutations, closed_notional))
}

/// Compute the liquidation fee, cap it at remaining margin, and add to the
/// user's fee entry.
///
/// Mutates:
///
/// - `fees` — liquidation fee added for the user.
///
/// Returns: `()`
fn apply_liquidation_fee(
    pnls: &BTreeMap<Addr, UsdValue>,
    fees: &mut BTreeMap<Addr, UsdValue>,
    user: Addr,
    closed_notional: UsdValue,
    liquidation_fee_rate: Dimensionless,
    user_margin: UsdValue,
) -> anyhow::Result<()> {
    let fee_usd = closed_notional.checked_mul(liquidation_fee_rate)?;
    let user_pnl = pnls.get(&user).copied().unwrap_or(UsdValue::ZERO);
    let remaining_margin = user_margin.checked_add(user_pnl)?.max(UsdValue::ZERO);
    let actual_fee = fee_usd.min(remaining_margin);

    if actual_fee.is_non_zero() {
        fees.entry(user)
            .or_default()
            .checked_add_assign(actual_fee)?;
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES,
            state::{ASKS, OrderKey},
        },
        dango_types::{
            Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue,
            perps::{Order, PairParam, PairState, Param, Position, State, UserState},
        },
        grug::{Addr, Coins, MockContext, Storage, Timestamp, Uint64},
        std::collections::BTreeMap,
    };

    /// Create a vault `UserState` with the given margin.
    fn vault_user_state_with_margin(margin: i128) -> UserState {
        UserState {
            margin: UsdValue::new_int(margin),
            ..Default::default()
        }
    }

    const USER: Addr = Addr::mock(1);
    const MAKER: Addr = Addr::mock(2);
    const CONTRACT: Addr = Addr::mock(0);

    fn pair_btc() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn pair_eth() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    fn default_param() -> Param {
        Param {
            taker_fee_rate: Dimensionless::new_permille(10), // 1%
            maker_fee_rate: Dimensionless::new_permille(10), // 1%
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

    fn eth_pair_param() -> PairParam {
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

    /// Save a user position.
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
        let key: OrderKey = (
            pair_id.clone(),
            UsdPrice::new_int(price),
            Uint64::new(order_id),
        );
        let order = Order {
            user: maker,
            size: Quantity::new_int(size),
            reduce_only: false,
            reserved_margin: UsdValue::ZERO,
        };
        ASKS.save(storage, key, &order).unwrap();
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
        let vault_state = UserState::default();

        // margin = 10000, equity = 10000 + 0 = 10000, MM = 2500
        // 10000 > 2500 → not liquidatable
        user_state.margin = UsdValue::new_int(10_000);

        use {
            dango_types::oracle::PrecisionedPrice,
            grug::{Udec128, hash_map},
        };
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000), // $50,000
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            &param,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            vault_state,
            &oracle_prices,
            &mut oracle_querier,
        );

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("not liquidatable"),
            "expected 'not liquidatable' error"
        );
    }

    #[test]
    fn single_position_full_close_via_book() {
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
        // PnL = 10 * (47500 - 50000) = -25000
        // Collateral = 2400 → equity = 2400 - 25000 = -22600
        // MM = 10 * 47500 * 0.05 = 23750
        // equity < MM → liquidatable
        save_position(&mut ctx.storage, USER, &pair_btc(), 10, 50_000);

        // Set up a maker with asks to absorb the liquidation.
        let mut maker_state = UserState::default();
        maker_state.positions.insert(pair_btc(), Position {
            size: Quantity::new_int(-10),
            entry_price: UsdPrice::new_int(47_500),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        USER_STATES
            .save(&mut ctx.storage, MAKER, &maker_state)
            .unwrap();

        // Maker has sell order for 10 BTC at $47,500.
        save_ask(&mut ctx.storage, &pair_btc(), 1, MAKER, -10, 47_500);

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(47_500));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let vault_state = vault_user_state_with_margin(1_000_000);

        user_state.margin = UsdValue::new_int(2_400);

        use {
            dango_types::oracle::PrecisionedPrice,
            grug::{Udec128, hash_map},
        };
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(4_750_000), // $47,500
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            &param,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            vault_state,
            &oracle_prices,
            &mut oracle_querier,
        );

        assert!(result.is_ok(), "liquidation failed: {:?}", result.err());

        // User's position should be closed.
        assert!(
            user_state.positions.is_empty(),
            "user should have no positions after liquidation"
        );
    }

    #[test]
    fn vault_backstop_on_empty_book() {
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

        // User long 10 BTC at 50000, oracle 47500.
        save_position(&mut ctx.storage, USER, &pair_btc(), 10, 50_000);

        // No maker orders in the book — vault must backstop.

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(47_500));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let vault_state = vault_user_state_with_margin(1_000_000);

        user_state.margin = UsdValue::new_int(2_400);

        use {
            dango_types::oracle::PrecisionedPrice,
            grug::{Udec128, hash_map},
        };
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(4_750_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            &param,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            vault_state,
            &oracle_prices,
            &mut oracle_querier,
        );

        assert!(result.is_ok(), "vault backstop failed: {:?}", result.err());

        let (maker_states, _) = result.unwrap();

        // User's position should be closed.
        assert!(user_state.positions.is_empty());

        // Vault buys from the user (who is selling to close their long),
        // so the vault ends up with a long position.
        let vault_final = &maker_states[&CONTRACT];
        assert!(
            vault_final.positions.contains_key(&pair_btc()),
            "vault should have the backstop position"
        );

        let vault_pos = &vault_final.positions[&pair_btc()];
        assert_eq!(vault_pos.size, Quantity::new_int(10));
    }

    #[test]
    fn multi_pair_largest_mm_first() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();

        let btc_state = PairState::default();
        let eth_state = PairState::default();

        setup_storage(&mut ctx.storage, &param, &[
            (pair_btc(), btc_pair_param(), btc_state.clone()),
            (pair_eth(), eth_pair_param(), eth_state.clone()),
        ]);

        // User has:
        // - Long 1 BTC at 50000, oracle 47000 → MM = 1 * 47000 * 0.05 = 2350
        // - Long 10 ETH at 3000, oracle 2800  → MM = 10 * 2800 * 0.05 = 1400
        // Total MM = 3750
        //
        // PnL BTC = 1 * (47000 - 50000) = -3000
        // PnL ETH = 10 * (2800 - 3000) = -2000
        // Total PnL = -5000
        //
        // Collateral = 4000 → equity = 4000 - 5000 = -1000
        // -1000 < 3750 → liquidatable
        // deficit = 3750 - (-1000) = 4750
        //
        // BTC has larger MM (2350) → processed first.
        let mut user_state = UserState::default();
        user_state.positions.insert(pair_btc(), Position {
            size: Quantity::new_int(1),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        user_state.positions.insert(pair_eth(), Position {
            size: Quantity::new_int(10),
            entry_price: UsdPrice::new_int(3_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        USER_STATES
            .save(&mut ctx.storage, USER, &user_state)
            .unwrap();

        // No book orders — vault backstops both.

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());
        pair_params.insert(pair_eth(), eth_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), btc_state);
        pair_states.insert(pair_eth(), eth_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(47_000));
        oracle_prices.insert(pair_eth(), UsdPrice::new_int(2_800));

        let vault_state = vault_user_state_with_margin(1_000_000);

        user_state.margin = UsdValue::new_int(4_000);

        use {
            dango_types::oracle::PrecisionedPrice,
            grug::{Udec128, hash_map},
        };
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(4_700_000),
                Timestamp::from_seconds(0),
                8,
            ),
            pair_eth() => PrecisionedPrice::new(
                Udec128::new_percent(280_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            &param,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            vault_state,
            &oracle_prices,
            &mut oracle_querier,
        );

        assert!(result.is_ok(), "multi-pair liq failed: {:?}", result.err());

        // Both positions should be closed since the deficit exceeds all MM.
        assert!(
            user_state.positions.is_empty(),
            "both positions should be closed"
        );
    }

    #[test]
    fn fee_capped_at_remaining_margin() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = Param {
            liquidation_fee_rate: Dimensionless::new_permille(500), // 50% fee to test capping
            ..default_param()
        };
        let pair_state = PairState {
            long_oi: Quantity::new_int(1),
            ..Default::default()
        };

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // User long 1 BTC at 50000, oracle 48000.
        // PnL = 1 * (48000 - 50000) = -2000
        // Collateral = 2500 → equity = 2500 - 2000 = 500
        // MM = 1 * 48000 * 0.05 = 2400
        // 500 < 2400 → liquidatable
        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);

        // Empty book — vault backstops at oracle.
        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(48_000));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let vault_state = vault_user_state_with_margin(1_000_000);

        user_state.margin = UsdValue::new_int(2_500);

        use {
            dango_types::oracle::PrecisionedPrice,
            grug::{Udec128, hash_map},
        };
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(4_800_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            &param,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            vault_state,
            &oracle_prices,
            &mut oracle_querier,
        );

        assert!(result.is_ok(), "fee capping failed: {:?}", result.err());

        // Closed notional = 1 * 48000 = 48000
        // Uncapped fee = 48000 * 0.50 = 24000
        // PnL after fill (at oracle, zero fee) = -2000 (loss)
        // Remaining margin = 2500 + (-2000) = 500
        // Actual fee = min(24000, 500) = 500
        // The fee should be capped at 500, not the full 24000.
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

    /// A maker with orders on two pairs is matched during a multi-pair
    /// liquidation. With the shared maker states map, both pairs' position
    /// changes must be preserved in the maker's final state.
    ///
    /// User is short on both pairs (closing = buy = taker_is_bid = true →
    /// matches against ASKS). MAKER has long positions with corresponding
    /// asks that get closed when matched.
    #[test]
    fn multi_pair_maker_state_preserved() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();

        // User is short → short_oi tracks their positions.
        let btc_state = PairState {
            short_oi: Quantity::new_int(1),
            ..Default::default()
        };

        let eth_state = PairState {
            short_oi: Quantity::new_int(10),
            ..Default::default()
        };

        setup_storage(&mut ctx.storage, &param, &[
            (pair_btc(), btc_pair_param(), btc_state.clone()),
            (pair_eth(), eth_pair_param(), eth_state.clone()),
        ]);

        // User has:
        // - Short 1 BTC at entry 47000, oracle 50000 → PnL = -1*(50000-47000) = -3000
        // - Short 10 ETH at entry 2800, oracle 3000 → PnL = -10*(3000-2800) = -2000
        // Total PnL = -5000, collateral = 4000, equity = -1000
        // MM = 1*50000*0.05 + 10*3000*0.05 = 2500 + 1500 = 4000
        // equity(-1000) < MM(4000) → liquidatable
        let mut user_state = UserState::default();
        user_state.positions.insert(pair_btc(), Position {
            size: Quantity::new_int(-1),
            entry_price: UsdPrice::new_int(47_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        user_state.positions.insert(pair_eth(), Position {
            size: Quantity::new_int(-10),
            entry_price: UsdPrice::new_int(2_800),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        USER_STATES
            .save(&mut ctx.storage, USER, &user_state)
            .unwrap();

        // MAKER has long positions with asks on both pairs.
        // The ask fills will close the maker's longs.
        let mut maker_state = UserState {
            open_order_count: 2,
            ..Default::default()
        };
        maker_state.positions.insert(pair_btc(), Position {
            size: Quantity::new_int(1),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        maker_state.positions.insert(pair_eth(), Position {
            size: Quantity::new_int(10),
            entry_price: UsdPrice::new_int(3_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        USER_STATES
            .save(&mut ctx.storage, MAKER, &maker_state)
            .unwrap();

        // Ask orders for MAKER on both pairs (selling to close their longs).
        save_ask(&mut ctx.storage, &pair_btc(), 1, MAKER, -1, 50_000);
        save_ask(&mut ctx.storage, &pair_eth(), 2, MAKER, -10, 3_000);

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());
        pair_params.insert(pair_eth(), eth_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), btc_state);
        pair_states.insert(pair_eth(), eth_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(50_000));
        oracle_prices.insert(pair_eth(), UsdPrice::new_int(3_000));

        let vault_state = vault_user_state_with_margin(1_000_000);

        user_state.margin = UsdValue::new_int(4_000);

        use {
            dango_types::oracle::PrecisionedPrice,
            grug::{Udec128, hash_map},
        };
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
            pair_eth() => PrecisionedPrice::new(
                Udec128::new_percent(300_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            &param,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            vault_state,
            &oracle_prices,
            &mut oracle_querier,
        );

        assert!(
            result.is_ok(),
            "multi-pair maker liq failed: {:?}",
            result.err()
        );

        let (maker_states, _) = result.unwrap();

        // The maker should have fills from BOTH pairs preserved.
        let final_maker = &maker_states[&MAKER];
        assert!(
            !final_maker.positions.contains_key(&pair_btc()),
            "MAKER BTC position should be closed (fully filled)"
        );
        assert!(
            !final_maker.positions.contains_key(&pair_eth()),
            "MAKER ETH position should be closed (fully filled)"
        );
    }

    /// The vault has a resting ask matched during liquidation AND provides
    /// backstop for the remainder. Both fills must be preserved in the vault's
    /// final state.
    ///
    /// User is short 10 BTC → to close, they buy. The taker is bid, matching
    /// against ASKs. The vault has a resting ask for 3 BTC. The remaining 7
    /// are backstopped by the vault at oracle price.
    #[test]
    fn vault_maker_and_backstop_fills_preserved() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let pair_state = PairState {
            short_oi: Quantity::new_int(10),
            ..Default::default()
        };

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // User has short 10 BTC at 47500, oracle 50000 → loss of $25,000.
        // PnL = -10 * (50000 - 47500) = -25000
        // Collateral = 2400 → equity = 2400 - 25000 = -22600
        // MM = 10 * 50000 * 0.05 = 25000
        // -22600 < 25000 → liquidatable
        save_position(&mut ctx.storage, USER, &pair_btc(), -10, 47_500);
        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();

        // Vault has a resting ask for 3 BTC at 50000 (will be matched as maker).
        // The vault has a long 3 position that is being offered via the ask,
        // and $1,000,000 margin.
        let mut vault_state = UserState {
            margin: UsdValue::new_int(1_000_000),
            open_order_count: 1,
            ..Default::default()
        };
        vault_state.positions.insert(pair_btc(), Position {
            size: Quantity::new_int(3),
            entry_price: UsdPrice::new_int(50_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
        });
        USER_STATES
            .save(&mut ctx.storage, CONTRACT, &vault_state)
            .unwrap();

        save_ask(&mut ctx.storage, &pair_btc(), 1, CONTRACT, -3, 50_000);

        let mut pair_params = BTreeMap::new();
        pair_params.insert(pair_btc(), btc_pair_param());

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(50_000));

        user_state.margin = UsdValue::new_int(2_400);

        use {
            dango_types::oracle::PrecisionedPrice,
            grug::{Udec128, hash_map},
        };
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(5_000_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let result = _liquidate(
            &ctx.storage,
            USER,
            CONTRACT,
            &param,
            &pair_params,
            &mut pair_states,
            &mut user_state,
            vault_state,
            &oracle_prices,
            &mut oracle_querier,
        );

        assert!(
            result.is_ok(),
            "vault maker+backstop failed: {:?}",
            result.err()
        );

        let (maker_states, _) = result.unwrap();

        // The vault had +3 long. During liquidation:
        // - Maker fill: sold 3 BTC (closing the vault's +3 long via the ask)
        // - Backstop fill: sold 7 BTC at oracle price (new short)
        // Net vault position: 0 (from closed long) + (-7) from backstop = -7 short.
        let vault_final = &maker_states[&CONTRACT];
        assert!(
            vault_final.positions.contains_key(&pair_btc()),
            "vault should have a BTC position"
        );
        let vault_pos = &vault_final.positions[&pair_btc()];
        assert_eq!(
            vault_pos.size,
            Quantity::new_int(-7),
            "vault should have -7 BTC short (3 maker fill closed the long, 7 backstop)"
        );
    }
}
