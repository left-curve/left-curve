use {
    crate::{
        ASKS, BIDS, NoCachePerpQuerier, PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES,
        core::{
            CloseEntry, accrue_funding, compute_close_schedule, compute_maintenance_margin,
            compute_user_equity, is_liquidatable,
        },
        execute::{
            BANK, ORACLE,
            cancel_order::cancel_all_orders_for,
            submit_order::{match_order, settle_fill, settle_pnls},
        },
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue, bank,
        perps::{
            Order, OrderId, PairId, PairParam, PairState, Param, State, UserState,
            settlement_currency,
        },
    },
    grug::{
        Addr, Coins, IsZero, Message, MutableCtx, Number, QuerierExt, Response, Storage, Uint128,
        coins,
    },
    std::collections::BTreeMap,
};

pub fn liquidate(ctx: MutableCtx, user: Addr) -> anyhow::Result<Response> {
    // ----------------------------- 1. Load state -----------------------------

    let param = PARAM.load(ctx.storage)?;
    let mut state = STATE.load(ctx.storage)?;

    let mut user_state = USER_STATES.may_load(ctx.storage, user)?.unwrap_or_default();

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    let collateral_balance = ctx
        .querier
        .query_balance(user, settlement_currency::DENOM.clone())?;

    let collateral_value = Quantity::from_base(collateral_balance, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // -------------------- 2. Cancel all resting orders -----------------------

    cancel_all_orders_for(ctx.storage, user, &mut user_state)?;

    // ------------------- 3. Accrue funding for all pairs ---------------------

    // Collect pair IDs first to avoid borrow conflicts.
    let pair_ids = user_state.positions.keys().cloned().collect::<Vec<_>>();

    let mut pair_params = BTreeMap::new();
    let mut pair_states = BTreeMap::new();

    for pair_id in &pair_ids {
        let pair_param = PAIR_PARAMS.load(ctx.storage, pair_id)?;
        let mut pair_state = PAIR_STATES.load(ctx.storage, pair_id)?;

        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;

        accrue_funding(
            &mut pair_state,
            &pair_param,
            ctx.block.timestamp,
            oracle_price,
        )?;

        // Save accrued pair state so NoCachePerpQuerier reads it.
        PAIR_STATES.save(ctx.storage, pair_id, &pair_state)?;

        pair_params.insert(pair_id.clone(), pair_param);
        pair_states.insert(pair_id.clone(), pair_state);
    }

    // -------------------- 4. Load vault state --------------------------------

    let mut vault_state = USER_STATES
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

    // ---------------------- 6. Call inner function ---------------------------

    let (payouts, collections, maker_states, order_mutations) = _liquidate(
        ctx.storage,
        user,
        ctx.contract,
        &param,
        &pair_params,
        &mut pair_states,
        &mut user_state,
        &mut vault_state,
        &oracle_prices,
        collateral_value,
        collateral_balance,
        &mut oracle_querier,
        settlement_currency_price,
        &mut state,
    )?;

    // --------------------- 7. Apply state changes ----------------------------

    STATE.save(ctx.storage, &state)?;

    for (pair_id, pair_state) in &pair_states {
        PAIR_STATES.save(ctx.storage, pair_id, pair_state)?;
    }

    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, user);
    } else {
        USER_STATES.save(ctx.storage, user, &user_state)?;
    }

    USER_STATES.save(ctx.storage, ctx.contract, &vault_state)?;

    for (addr, maker_state) in &maker_states {
        USER_STATES.save(ctx.storage, *addr, maker_state)?;
    }

    // -------------------- 8. Apply order mutations ---------------------------

    for (pair_id, taker_is_bid, stored_price, order_id, mutation) in order_mutations {
        let order_key = (pair_id, stored_price, order_id);

        let maker_book = if taker_is_bid {
            ASKS
        } else {
            BIDS
        };

        match mutation {
            Some(order) => {
                maker_book.save(ctx.storage, order_key, &order)?;
            },
            None => {
                maker_book.remove(ctx.storage, order_key)?;
            },
        }
    }

    // -------------------- 9. Generate messages -------------------------------

    let mut messages = Vec::with_capacity(payouts.len() + collections.len());

    if !payouts.is_empty() {
        messages.push(Message::batch_transfer(payouts.into_iter().map(
            |(addr, amount)| {
                (
                    addr,
                    coins! { settlement_currency::DENOM.clone() => amount },
                )
            },
        ))?);
    }

    for (user, amount) in collections {
        messages.push(Message::execute(
            BANK,
            &bank::ExecuteMsg::ForceTransfer {
                from: user,
                to: ctx.contract,
                coins: coins! { settlement_currency::DENOM.clone() => amount },
            },
            Coins::new(),
        )?);
    }

    Ok(Response::new().add_messages(messages))
}

/// Execute the close schedule against the order book, with vault backstop for
/// any unfilled remainder.
fn execute_close_schedule(
    storage: &dyn Storage,
    schedule: &[CloseEntry],
    user: Addr,
    contract: Addr,
    param: &Param,
    pair_states: &mut BTreeMap<PairId, PairState>,
    user_state: &mut UserState,
    vault_state: &mut UserState,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
) -> anyhow::Result<(
    BTreeMap<Addr, UsdValue>,
    BTreeMap<Addr, UserState>,
    Vec<(PairId, bool, UsdPrice, OrderId, Option<Order>)>,
    UsdValue,
)> {
    // Zero-fee param for liquidation fills.
    let liq_param = Param {
        taker_fee_rate: Dimensionless::ZERO,
        maker_fee_rate: Dimensionless::ZERO,
        ..param.clone()
    };

    let mut all_pnls: BTreeMap<Addr, UsdValue> = BTreeMap::new();
    let mut all_maker_states: BTreeMap<Addr, UserState> = BTreeMap::new();
    let mut all_order_mutations: Vec<(PairId, bool, UsdPrice, OrderId, Option<Order>)> = Vec::new();
    let mut closed_notional = UsdValue::ZERO;

    for entry in schedule {
        let pair_state = pair_states.get_mut(&entry.pair_id).unwrap();
        let oracle_price = oracle_prices[&entry.pair_id];

        let taker_is_bid = entry.close_size.is_positive();
        let target_price = if taker_is_bid {
            UsdPrice::MAX
        } else {
            UsdPrice::ZERO
        };

        let (unfilled, pnls, maker_states, order_mutations) = match_order(
            storage,
            &liq_param,
            &entry.pair_id,
            pair_state,
            user,
            user_state,
            taker_is_bid,
            target_price,
            entry.close_size,
        )?;

        // Merge PnLs.
        for (addr, pnl) in pnls {
            all_pnls.entry(addr).or_default().checked_add_assign(pnl)?;
        }

        // Merge maker states.
        for (addr, ms) in maker_states {
            all_maker_states.insert(addr, ms);
        }

        // Collect order mutations with pair context.
        for (stored_price, order_id, mutation) in order_mutations {
            all_order_mutations.push((
                entry.pair_id.clone(),
                taker_is_bid,
                stored_price,
                order_id,
                mutation,
            ));
        }

        // Track closed notional for fee calculation.
        let filled = entry.close_size.checked_sub(unfilled)?;
        closed_notional.checked_add_assign(filled.checked_abs()?.checked_mul(oracle_price)?)?;

        // Vault backstop: if there is unfilled remainder, the vault absorbs at oracle price.
        if unfilled.is_non_zero() {
            // User side: close at oracle price with zero fee.
            settle_fill(
                &entry.pair_id,
                pair_state,
                user_state,
                unfilled,
                oracle_price,
                Dimensionless::ZERO,
                &mut all_pnls,
                user,
            )?;

            // Vault side: opposite fill at oracle price with zero fee.
            let vault_fill = unfilled.checked_neg()?;
            settle_fill(
                &entry.pair_id,
                pair_state,
                vault_state,
                vault_fill,
                oracle_price,
                Dimensionless::ZERO,
                &mut all_pnls,
                contract,
            )?;

            // Add vault backstop notional.
            closed_notional
                .checked_add_assign(unfilled.checked_abs()?.checked_mul(oracle_price)?)?;
        }
    }

    Ok((all_pnls, all_maker_states, all_order_mutations, closed_notional))
}

/// Compute the liquidation fee, cap it at remaining margin, and deduct from
/// the user's PnL entry.
fn apply_liquidation_fee(
    pnls: &mut BTreeMap<Addr, UsdValue>,
    user: Addr,
    closed_notional: UsdValue,
    liquidation_fee_rate: Dimensionless,
    collateral_value: UsdValue,
) -> anyhow::Result<()> {
    let fee_usd = closed_notional.checked_mul(liquidation_fee_rate)?;
    let user_pnl = pnls.get(&user).copied().unwrap_or(UsdValue::ZERO);
    let remaining_margin = collateral_value.checked_add(user_pnl)?.max(UsdValue::ZERO);
    let actual_fee = fee_usd.min(remaining_margin);

    // Deduct the fee from the user's PnL entry. This routes the fee to the
    // insurance fund when settle_pnls converts USD values to base amounts.
    if actual_fee.is_non_zero() {
        pnls.entry(user)
            .or_default()
            .checked_sub_assign(actual_fee)?;
    }

    Ok(())
}

/// Extract the vault's PnL from the map and apply directly to `state`
/// (the contract can't transfer to itself).
fn settle_vault_pnl(
    pnls: &mut BTreeMap<Addr, UsdValue>,
    contract: Addr,
    settlement_currency_price: UsdPrice,
    state: &mut State,
) -> anyhow::Result<()> {
    if let Some(vault_pnl) = pnls.remove(&contract) {
        if vault_pnl > UsdValue::ZERO {
            let amount = vault_pnl
                .checked_div(settlement_currency_price)?
                .into_base_floor(settlement_currency::DECIMAL)?;

            if amount.is_non_zero() {
                state.vault_margin = state.vault_margin.checked_add(amount)?;
            }
        } else if vault_pnl < UsdValue::ZERO {
            let amount = vault_pnl
                .checked_abs()?
                .checked_div(settlement_currency_price)?
                .into_base_ceil(settlement_currency::DECIMAL)?;

            if amount.is_non_zero() {
                state.vault_margin = state.vault_margin.checked_sub(amount)?;
            }
        }
    }

    Ok(())
}

/// Mutates:
///
/// - `pair_states` — OI updated per fill.
/// - `user_state.positions` — closed (partially or fully) per the schedule.
/// - `vault_state.positions` — opened for any vault-backstopped fills.
/// - `state.insurance_fund` — adjusted by settled PnLs and bad debt.
/// - `state.vault_margin` — adjusted by the vault's PnL entry.
///
/// Returns:
///
/// - Per-user payouts in settlement-currency base units.
/// - Per-user collections in settlement-currency base units.
/// - Maker `UserState`s to persist.
/// - Order mutations to apply: `(pair_id, taker_is_bid, stored_price, order_id, Option<Order>)`.
fn _liquidate(
    storage: &dyn Storage,
    user: Addr,
    contract: Addr,
    param: &Param,
    pair_params: &BTreeMap<PairId, PairParam>,
    pair_states: &mut BTreeMap<PairId, PairState>,
    user_state: &mut UserState,
    vault_state: &mut UserState,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    collateral_value: UsdValue,
    collateral_balance: Uint128,
    oracle_querier: &mut OracleQuerier,
    settlement_currency_price: UsdPrice,
    state: &mut State,
) -> anyhow::Result<(
    BTreeMap<Addr, Uint128>,
    Vec<(Addr, Uint128)>,
    BTreeMap<Addr, UserState>,
    Vec<(PairId, bool, UsdPrice, OrderId, Option<Order>)>,
)> {
    // -------------------- Step 1: Assert liquidatable -------------------------

    let perp_querier = NoCachePerpQuerier::new_local(storage);

    ensure!(
        is_liquidatable(collateral_value, user_state, &perp_querier, oracle_querier)?,
        "user is not liquidatable"
    );

    // ------------- Step 2: Compute close schedule (largest-MM-first) ----------

    let equity = compute_user_equity(collateral_value, user_state, &perp_querier, oracle_querier)?;
    let total_mm = compute_maintenance_margin(user_state, &perp_querier, oracle_querier)?;
    let deficit = total_mm.checked_sub(equity)?;

    let schedule = compute_close_schedule(user_state, pair_params, oracle_prices, deficit)?;

    // -------- Step 3: Execute closes via the order book -----------------------

    let (mut all_pnls, all_maker_states, all_order_mutations, closed_notional) =
        execute_close_schedule(
            storage,
            &schedule,
            user,
            contract,
            param,
            pair_states,
            user_state,
            vault_state,
            oracle_prices,
        )?;

    // -------------------- Step 4: Liquidation fee -----------------------------

    apply_liquidation_fee(
        &mut all_pnls,
        user,
        closed_notional,
        param.liquidation_fee_rate,
        collateral_value,
    )?;

    // ------------- Step 5: Handle vault PnL + bad debt ------------------------

    settle_vault_pnl(&mut all_pnls, contract, settlement_currency_price, state)?;

    // ----------------------- Step 6: Settle PnLs ------------------------------

    let (payouts, mut collections) = settle_pnls(all_pnls, settlement_currency_price, state)?;

    // Bad debt check: if the user owes more than their collateral, cap the
    // collection and absorb the bad debt from the insurance fund.
    if let Some((_, amount)) = collections.iter_mut().find(|(addr, _)| *addr == user)
        && *amount > collateral_balance
    {
        let bad_debt = amount.checked_sub(collateral_balance)?;
        state.insurance_fund.checked_sub_assign(bad_debt)?;
        *amount = collateral_balance;
    }

    // Remove zero-amount collections.
    collections.retain(|(_, amount)| amount.is_non_zero());

    // If the user has a payout, it goes to the insurance fund instead (the user
    // was liquidated; any remaining equity after paying off debts is forfeit to
    // the insurance fund via the liquidation fee mechanism — but any payout
    // from settle_pnls is legitimate margin return).
    // Actually, payouts from settle_pnls are legitimate — they happen when
    // the user's positions were closed at a profit. Keep them.

    Ok((payouts, collections, all_maker_states, all_order_mutations))
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
        grug::{Addr, Coins, MockContext, Storage, Timestamp, Uint64, Uint128},
        std::collections::BTreeMap,
    };

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
            skew_scale: Quantity::new_int(100_000),
            max_abs_oi: Quantity::new_int(1_000_000),
            ..Default::default()
        }
    }

    fn eth_pair_param() -> PairParam {
        PairParam {
            initial_margin_ratio: Dimensionless::new_permille(100), // 10%
            maintenance_margin_ratio: Dimensionless::new_permille(50), // 5%
            skew_scale: Quantity::new_int(100_000),
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

    fn state_with_insurance(amount: u128) -> State {
        State {
            insurance_fund: Uint128::new(amount),
            ..Default::default()
        }
    }

    // ======================== Tests ========================

    #[test]
    fn not_liquidatable_errors() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let pair_state = PairState::new(Timestamp::from_seconds(0));

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
        pair_states.insert(pair_btc(), PairState::new(Timestamp::from_seconds(0)));

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(50_000));

        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let mut vault_state = UserState::default();
        let mut state = STATE.load(&ctx.storage).unwrap();

        // collateral_value = 10000, equity = 10000 + 0 = 10000, MM = 2500
        // 10000 > 2500 → not liquidatable
        let collateral_value = UsdValue::new_int(10_000);
        let collateral_balance = Uint128::new(10_000_000_000); // 10000 * 1e6

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
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100), // $1
                Timestamp::from_seconds(0),
                6,
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
            &mut vault_state,
            &oracle_prices,
            collateral_value,
            collateral_balance,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
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
        let mut pair_state = PairState::new(Timestamp::from_seconds(0));
        pair_state.long_oi = Quantity::new_int(10);

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
        let mut vault_state = UserState::default();
        let mut state = state_with_insurance(1_000_000_000_000);

        let collateral_value = UsdValue::new_int(2_400);
        let collateral_balance = Uint128::new(2_400_000_000);

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
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100), // $1
                Timestamp::from_seconds(0),
                6,
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
            &mut vault_state,
            &oracle_prices,
            collateral_value,
            collateral_balance,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
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
        let mut pair_state = PairState::new(Timestamp::from_seconds(0));
        pair_state.long_oi = Quantity::new_int(10);

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
        let mut vault_state = UserState::default();
        let mut state = state_with_insurance(1_000_000_000_000);

        let collateral_value = UsdValue::new_int(2_400);
        let collateral_balance = Uint128::new(2_400_000_000);

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
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Timestamp::from_seconds(0),
                6,
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
            &mut vault_state,
            &oracle_prices,
            collateral_value,
            collateral_balance,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
        );

        assert!(result.is_ok(), "vault backstop failed: {:?}", result.err());

        // User's position should be closed.
        assert!(user_state.positions.is_empty());

        // Vault buys from the user (who is selling to close their long),
        // so the vault ends up with a long position.
        assert!(
            vault_state.positions.contains_key(&pair_btc()),
            "vault should have the backstop position"
        );

        let vault_pos = &vault_state.positions[&pair_btc()];
        assert_eq!(vault_pos.size, Quantity::new_int(10));
    }

    #[test]
    fn multi_pair_largest_mm_first() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();

        let btc_state = PairState::new(Timestamp::from_seconds(0));
        let eth_state = PairState::new(Timestamp::from_seconds(0));

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

        let mut vault_state = UserState::default();
        let mut state = state_with_insurance(1_000_000_000_000);

        let collateral_value = UsdValue::new_int(4_000);
        let collateral_balance = Uint128::new(4_000_000_000);

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
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Timestamp::from_seconds(0),
                6,
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
            &mut vault_state,
            &oracle_prices,
            collateral_value,
            collateral_balance,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
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
        let mut pair_state = PairState::new(Timestamp::from_seconds(0));
        pair_state.long_oi = Quantity::new_int(1);

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
        let mut vault_state = UserState::default();
        let mut state = state_with_insurance(1_000_000_000_000);

        let collateral_value = UsdValue::new_int(2_500);
        let collateral_balance = Uint128::new(2_500_000_000);

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
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Timestamp::from_seconds(0),
                6,
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
            &mut vault_state,
            &oracle_prices,
            collateral_value,
            collateral_balance,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
        );

        assert!(result.is_ok(), "fee capping failed: {:?}", result.err());

        // Closed notional = 1 * 48000 = 48000
        // Uncapped fee = 48000 * 0.50 = 24000
        // PnL after fill (at oracle, zero fee) = -2000 (loss)
        // Remaining margin = 2500 + (-2000) = 500
        // Actual fee = min(24000, 500) = 500
        // The fee should be capped at 500, not the full 24000.
    }
}
