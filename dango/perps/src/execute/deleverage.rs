use {
    crate::{
        NoCachePerpQuerier, PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES,
        core::{accrue_funding, compute_adl_score, compute_user_equity},
        execute::{ORACLE, cancel_order::cancel_all_orders_for, submit_order::settle_fill},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, Quantity, UsdPrice, UsdValue,
        perps::{PairId, PairState, State, UserState, settlement_currency},
    },
    grug::{Addr, IsZero, Message, MutableCtx, Number, QuerierExt, Response, Uint128, coins},
    std::collections::BTreeMap,
};

pub fn deleverage(ctx: MutableCtx, user: Addr) -> anyhow::Result<Response> {
    // ----------------------------- 1. Load state + checks -----------------------

    let param = PARAM.load(ctx.storage)?;

    ensure!(
        param.adl_operators.contains(&ctx.sender),
        "sender is not an authorized ADL operator"
    );

    let mut state = STATE.load(ctx.storage)?;

    ensure!(state.adl_deficit.is_non_zero(), "no ADL deficit");

    let mut user_state = USER_STATES.may_load(ctx.storage, user)?.unwrap_or_default();

    ensure!(!user_state.positions.is_empty(), "user has no positions");

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

    let pair_ids = user_state.positions.keys().cloned().collect::<Vec<_>>();

    let mut pair_states = BTreeMap::new();
    let mut oracle_prices = BTreeMap::new();

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

        pair_states.insert(pair_id.clone(), pair_state);
        oracle_prices.insert(pair_id.clone(), oracle_price);
    }

    // -------------------- 4–6. Core ADL logic --------------------------------

    let payout = _deleverage(
        ctx.storage,
        user,
        &mut pair_states,
        &mut user_state,
        &oracle_prices,
        collateral_value,
        &mut oracle_querier,
        settlement_currency_price,
        &mut state,
    )?;

    // -------------------- 7. Apply state changes -----------------------------

    STATE.save(ctx.storage, &state)?;

    for (pair_id, pair_state) in &pair_states {
        PAIR_STATES.save(ctx.storage, pair_id, pair_state)?;
    }

    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, user);
    } else {
        USER_STATES.save(ctx.storage, user, &user_state)?;
    }

    // -------------------- 8. Generate messages -------------------------------

    let mut messages = Vec::new();

    if let Some((addr, amount)) = payout {
        messages.push(Message::transfer(
            addr,
            coins! { settlement_currency::DENOM.clone() => amount },
        )?);
    }

    Ok(Response::new().add_messages(messages))
}

/// Core ADL logic: rank positions, close profitable ones, handle forfeiture.
///
/// Mutates:
///
/// - `pair_states` — OI updated per fill.
/// - `user_state.positions` — profitable positions closed.
/// - `state.vault_margin` — restocked from forfeited PnL.
/// - `state.adl_deficit` — reduced by forfeited amount.
///
/// Returns an optional payout `(user, amount)` in settlement-currency base units.
fn _deleverage(
    storage: &dyn grug::Storage,
    user: Addr,
    pair_states: &mut BTreeMap<PairId, PairState>,
    user_state: &mut UserState,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    collateral_value: UsdValue,
    oracle_querier: &mut OracleQuerier,
    settlement_currency_price: UsdPrice,
    state: &mut State,
) -> anyhow::Result<Option<(Addr, Uint128)>> {
    // -------------------- Step 1: Compute equity + rank ----------------------

    let perp_querier = NoCachePerpQuerier::new_local(storage);
    let user_equity =
        compute_user_equity(collateral_value, user_state, &perp_querier, oracle_querier)?;

    let mut scored: Vec<(PairId, Dimensionless)> = Vec::new();

    for (pair_id, position) in &user_state.positions {
        let oracle_price = oracle_prices[pair_id];
        let score = compute_adl_score(position, oracle_price, user_equity)?;

        if score > Dimensionless::ZERO {
            scored.push((pair_id.clone(), score));
        }
    }

    // Sort by score descending.
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    ensure!(!scored.is_empty(), "no profitable positions to ADL");

    // --------- Step 2: Close profitable positions at oracle price ------------

    let mut pnls: BTreeMap<Addr, UsdValue> = BTreeMap::new();

    for (pair_id, _score) in &scored {
        let pair_state = pair_states.get_mut(pair_id).unwrap();
        let oracle_price = oracle_prices[pair_id];

        // Get close size before settle_fill removes the position.
        let close_size = user_state
            .positions
            .get(pair_id)
            .unwrap()
            .size
            .checked_neg()?;

        settle_fill(
            pair_id,
            pair_state,
            user_state,
            close_size,
            oracle_price,
            Dimensionless::ZERO,
            &mut pnls,
            user,
        )?;
    }

    // --------- Step 3: Settle PnL with partial forfeiture --------------------

    let user_pnl = pnls.remove(&user).unwrap_or(UsdValue::ZERO);

    if user_pnl > UsdValue::ZERO {
        let pnl_quantity = user_pnl.checked_div(settlement_currency_price)?;
        let pnl_base = pnl_quantity.into_base_floor(settlement_currency::DECIMAL)?;

        // Restock vault with full PnL.
        state.vault_margin.checked_add_assign(pnl_base)?;

        // Forfeit up to the deficit.
        let forfeited = pnl_base.min(state.adl_deficit);
        let payout = pnl_base.checked_sub(forfeited)?;

        // Pay out the non-forfeited portion.
        state.vault_margin.checked_sub_assign(payout)?;
        state.adl_deficit.checked_sub_assign(forfeited)?;

        if payout.is_non_zero() {
            return Ok(Some((user, payout)));
        }
    }

    // If PnL is negative or zero, don't penalize the user further.
    // No deficit reduction (need another ADL target).

    Ok(None)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{PAIR_PARAMS, PAIR_STATES, PARAM, STATE, USER_STATES},
        dango_types::{
            Dimensionless, FundingPerUnit, Quantity, UsdPrice, UsdValue,
            oracle::PrecisionedPrice,
            perps::{PairId, PairParam, PairState, Param, Position, State, settlement_currency},
        },
        grug::{
            Addr, Coins, MockContext, NumberConst, Storage, Timestamp, Udec128, Uint128, hash_map,
        },
        std::collections::{BTreeMap, BTreeSet},
    };

    const CONTRACT: Addr = Addr::mock(0);

    const USER: Addr = Addr::mock(1);

    fn pair_btc() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    fn pair_eth() -> PairId {
        "perp/ethusd".parse().unwrap()
    }

    fn default_param() -> Param {
        Param {
            taker_fee_rate: Dimensionless::new_permille(10),
            maker_fee_rate: Dimensionless::new_permille(10),
            liquidation_fee_rate: Dimensionless::new_permille(10),
            max_open_orders: 100,
            adl_operators: BTreeSet::from([Addr::mock(99)]),
            ..Default::default()
        }
    }

    fn btc_pair_param() -> PairParam {
        PairParam {
            initial_margin_ratio: Dimensionless::new_permille(100),
            maintenance_margin_ratio: Dimensionless::new_permille(50),
            skew_scale: Quantity::new_int(100_000),
            max_abs_oi: Quantity::new_int(1_000_000),
            ..Default::default()
        }
    }

    fn eth_pair_param() -> PairParam {
        PairParam {
            initial_margin_ratio: Dimensionless::new_permille(100),
            maintenance_margin_ratio: Dimensionless::new_permille(50),
            skew_scale: Quantity::new_int(100_000),
            max_abs_oi: Quantity::new_int(1_000_000),
            ..Default::default()
        }
    }

    fn setup_storage(
        storage: &mut dyn Storage,
        param: &Param,
        state: &State,
        pairs: &[(PairId, PairParam, PairState)],
    ) {
        PARAM.save(storage, param).unwrap();
        STATE.save(storage, state).unwrap();

        for (pair_id, pair_param, pair_state) in pairs {
            PAIR_PARAMS.save(storage, pair_id, pair_param).unwrap();
            PAIR_STATES.save(storage, pair_id, pair_state).unwrap();
        }
    }

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

    fn state_with_deficit(vault_margin: u128, adl_deficit: u128) -> State {
        State {
            vault_margin: Uint128::new(vault_margin),
            adl_deficit: Uint128::new(adl_deficit),
            ..Default::default()
        }
    }

    // ======================== Public function tests ===========================

    #[test]
    fn no_deficit_errors() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_sender(Addr::mock(99))
            .with_funds(Coins::default());

        let param = default_param();
        let state = State::default(); // adl_deficit = 0

        setup_storage(&mut ctx.storage, &param, &state, &[(
            pair_btc(),
            btc_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
        )]);

        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);

        let result = super::deleverage(ctx.as_mutable(), USER);

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("no ADL deficit"),
            "expected 'no ADL deficit' error"
        );
    }

    #[test]
    fn user_no_positions_errors() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_sender(Addr::mock(99))
            .with_funds(Coins::default());

        let param = default_param();
        let state = state_with_deficit(0, 5_000_000_000);

        setup_storage(&mut ctx.storage, &param, &state, &[(
            pair_btc(),
            btc_pair_param(),
            PairState::new(Timestamp::from_seconds(0)),
        )]);

        // Don't save any positions for USER.

        let result = super::deleverage(ctx.as_mutable(), USER);

        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("no positions"),
            "expected 'no positions' error"
        );
    }

    // ======================== _deleverage tests ===============================

    #[test]
    fn no_profitable_positions_errors() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let mut state = state_with_deficit(0, 5_000_000_000);
        let pair_state = PairState::new(Timestamp::from_seconds(0));

        setup_storage(&mut ctx.storage, &param, &state, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // User has long 1 BTC at $50k, oracle at $45k → loss
        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);
        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(45_000));

        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(4_500_000),
                Timestamp::from_seconds(0),
                8,
            ),
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Timestamp::from_seconds(0),
                6,
            ),
        });

        let collateral_value = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            collateral_value,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("no profitable positions"),
            "expected 'no profitable positions' error"
        );
    }

    #[test]
    fn single_profitable_long_deficit_less_than_pnl() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        // Deficit = 5000 USDC (5_000_000_000 base units)
        let mut state = state_with_deficit(0, 5_000_000_000);
        let mut pair_state = PairState::new(Timestamp::from_seconds(0));
        pair_state.long_oi = Quantity::new_int(1);

        setup_storage(&mut ctx.storage, &param, &state, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // User has long 1 BTC at $50k, oracle at $65k → PnL = $15k
        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);
        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(65_000));

        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(6_500_000),
                Timestamp::from_seconds(0),
                8,
            ),
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Timestamp::from_seconds(0),
                6,
            ),
        });

        let collateral_value = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            collateral_value,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
        );

        assert!(result.is_ok(), "deleverage failed: {:?}", result.err());

        // Position should be removed.
        assert!(
            user_state.positions.is_empty(),
            "user should have no positions after ADL"
        );

        // OI should be reduced.
        assert_eq!(pair_states[&pair_btc()].long_oi, Quantity::ZERO);

        // PnL = 15000 USDC = 15_000_000_000 base
        // vault_margin += 15_000_000_000 (restock)
        // forfeited = min(15_000_000_000, 5_000_000_000) = 5_000_000_000
        // payout = 15_000_000_000 - 5_000_000_000 = 10_000_000_000
        // vault_margin -= 10_000_000_000 (payout)
        // net vault_margin = 0 + 15_000_000_000 - 10_000_000_000 = 5_000_000_000
        assert_eq!(state.vault_margin, Uint128::new(5_000_000_000));
        assert_eq!(state.adl_deficit, Uint128::ZERO);

        // Payout should be 10_000_000_000.
        let payout = result.unwrap();
        assert_eq!(payout, Some((USER, Uint128::new(10_000_000_000))));
    }

    #[test]
    fn single_profitable_long_deficit_greater_than_pnl() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        // Deficit = 20000 USDC, PnL will be 15000
        let mut state = state_with_deficit(0, 20_000_000_000);
        let mut pair_state = PairState::new(Timestamp::from_seconds(0));
        pair_state.long_oi = Quantity::new_int(1);

        setup_storage(&mut ctx.storage, &param, &state, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        // Long 1 BTC at $50k, oracle at $65k → PnL = $15k
        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);
        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(65_000));

        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(6_500_000),
                Timestamp::from_seconds(0),
                8,
            ),
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Timestamp::from_seconds(0),
                6,
            ),
        });

        let collateral_value = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            collateral_value,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
        );

        assert!(result.is_ok(), "deleverage failed: {:?}", result.err());

        // All PnL forfeited, no payout.
        assert_eq!(result.unwrap(), None);

        // vault_margin = 0 + 15_000_000_000 (restock) - 0 (no payout) = 15_000_000_000
        assert_eq!(state.vault_margin, Uint128::new(15_000_000_000));
        // deficit: 20_000_000_000 - 15_000_000_000 = 5_000_000_000
        assert_eq!(state.adl_deficit, Uint128::new(5_000_000_000));
    }

    #[test]
    fn mixed_positions_only_profitable_closed() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let mut state = state_with_deficit(0, 3_000_000_000);

        let btc_state = PairState::new(Timestamp::from_seconds(0));
        let eth_state = PairState::new(Timestamp::from_seconds(0));

        setup_storage(&mut ctx.storage, &param, &state, &[
            (pair_btc(), btc_pair_param(), btc_state.clone()),
            (pair_eth(), eth_pair_param(), eth_state.clone()),
        ]);

        // BTC: long 1 at $50k, oracle $55k → profit $5k
        // ETH: long 1 at $3000, oracle $2500 → loss $500
        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);
        save_position(&mut ctx.storage, USER, &pair_eth(), 1, 3_000);
        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), btc_state);
        pair_states.insert(pair_eth(), eth_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(55_000));
        oracle_prices.insert(pair_eth(), UsdPrice::new_int(2_500));

        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(5_500_000),
                Timestamp::from_seconds(0),
                8,
            ),
            pair_eth() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                8,
            ),
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Timestamp::from_seconds(0),
                6,
            ),
        });

        let collateral_value = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            collateral_value,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
        );

        assert!(result.is_ok(), "deleverage failed: {:?}", result.err());

        // BTC position closed (profitable), ETH kept (at loss).
        assert!(
            !user_state.positions.contains_key(&pair_btc()),
            "BTC position should be closed"
        );
        assert!(
            user_state.positions.contains_key(&pair_eth()),
            "ETH position should be kept"
        );
    }

    #[test]
    fn collateral_unchanged() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let mut state = state_with_deficit(0, 5_000_000_000);
        let mut pair_state = PairState::new(Timestamp::from_seconds(0));
        pair_state.long_oi = Quantity::new_int(1);

        setup_storage(&mut ctx.storage, &param, &state, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        save_position(&mut ctx.storage, USER, &pair_btc(), 1, 50_000);
        let mut user_state = USER_STATES.load(&ctx.storage, USER).unwrap();

        let mut pair_states = BTreeMap::new();
        pair_states.insert(pair_btc(), pair_state);

        let mut oracle_prices = BTreeMap::new();
        oracle_prices.insert(pair_btc(), UsdPrice::new_int(65_000));

        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            pair_btc() => PrecisionedPrice::new(
                Udec128::new_percent(6_500_000),
                Timestamp::from_seconds(0),
                8,
            ),
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(100),
                Timestamp::from_seconds(0),
                6,
            ),
        });

        let collateral_value = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            collateral_value,
            &mut oracle_querier,
            UsdPrice::new_int(1),
            &mut state,
        );

        assert!(result.is_ok(), "deleverage failed: {:?}", result.err());

        // _deleverage never force-collects from user. It only pays out from
        // the vault. The collateral is not touched.
        // (No collections returned — only an optional payout.)
    }
}
