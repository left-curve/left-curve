use {
    crate::{
        NoCachePerpQuerier, PAIR_STATES, PARAM, USER_STATES,
        core::{compute_adl_score, compute_user_equity},
        execute::{ORACLE, cancel_order::cancel_all_orders_for, submit_order::settle_fill},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, UsdPrice, UsdValue,
        perps::{PairId, PairState, UserState},
    },
    grug::{Addr, MutableCtx, Response},
    std::collections::BTreeMap,
};

/// Auto-deleverage profitable positions to reduce the vault's ADL deficit.
///
/// Mutates: `STATE`, `PAIR_STATES`, `USER_STATES`.
///
/// Returns: empty `Response` (all PnL settled via internal margins).
pub fn deleverage(ctx: MutableCtx, user: Addr) -> anyhow::Result<Response> {
    ensure!(user != ctx.contract, "cannot ADL the vault");

    // ----------------------------- 1. Load state + checks -----------------------

    let param = PARAM.load(ctx.storage)?;

    ensure!(
        param.adl_operators.contains(&ctx.sender),
        "sender is not an authorized ADL operator"
    );

    let mut vault_user_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    ensure!(vault_user_state.margin.is_negative(), "no ADL deficit");

    let mut user_state = USER_STATES.may_load(ctx.storage, user)?.unwrap_or_default();

    ensure!(!user_state.positions.is_empty(), "user has no positions");

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // -------------------- 2. Cancel all resting orders -----------------------

    cancel_all_orders_for(ctx.storage, user, &mut user_state)?;

    // ------------------- 3. Load pair states and oracle prices ----------------

    let pair_ids = user_state.positions.keys().cloned().collect::<Vec<_>>();

    let mut pair_states = BTreeMap::new();
    let mut oracle_prices = BTreeMap::new();

    for pair_id in &pair_ids {
        let pair_state = PAIR_STATES.load(ctx.storage, pair_id)?;
        let oracle_price = oracle_querier.query_price_for_perps(pair_id)?;

        pair_states.insert(pair_id.clone(), pair_state);
        oracle_prices.insert(pair_id.clone(), oracle_price);
    }

    // -------------------- 4–6. Core ADL logic --------------------------------

    _deleverage(
        ctx.storage,
        user,
        &mut pair_states,
        &mut user_state,
        &oracle_prices,
        &mut oracle_querier,
        &mut vault_user_state,
    )?;

    // -------------------- 7. Apply state changes -----------------------------

    for (pair_id, pair_state) in &pair_states {
        PAIR_STATES.save(ctx.storage, pair_id, pair_state)?;
    }

    if user_state.is_empty() {
        USER_STATES.remove(ctx.storage, user)?;
    } else {
        USER_STATES.save(ctx.storage, user, &user_state)?;
    }

    if vault_user_state.is_empty() {
        USER_STATES.remove(ctx.storage, ctx.contract)?;
    } else {
        USER_STATES.save(ctx.storage, ctx.contract, &vault_user_state)?;
    }

    // No token transfers — all PnL settled via internal margins.
    Ok(Response::new())
}

/// Core ADL logic: rank positions, close profitable ones, handle forfeiture.
///
/// Mutates:
///
/// - `pair_states` — OI updated per fill.
/// - `user_state.positions` — profitable positions closed.
/// - `user_state.margin` — credited with non-forfeited PnL.
/// - `vault_user_state.margin` — recovered toward zero from forfeited PnL.
///
/// Returns: `()`
fn _deleverage(
    storage: &dyn grug::Storage,
    user: Addr,
    pair_states: &mut BTreeMap<PairId, PairState>,
    user_state: &mut UserState,
    oracle_prices: &BTreeMap<PairId, UsdPrice>,
    oracle_querier: &mut OracleQuerier,
    vault_user_state: &mut UserState,
) -> anyhow::Result<()> {
    // -------------------- Step 1: Compute equity + rank ----------------------

    let perp_querier = NoCachePerpQuerier::new_local(storage);
    let user_equity = compute_user_equity(user_state, &perp_querier, oracle_querier)?;

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
    let mut fees: BTreeMap<Addr, UsdValue> = BTreeMap::new();

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
            &mut fees,
            user,
        )?;
    }

    // --------- Step 3: Settle PnL with partial forfeiture --------------------

    let user_pnl = pnls.remove(&user).unwrap_or(UsdValue::ZERO);

    if user_pnl > UsdValue::ZERO {
        // The deficit is the absolute value of the negative vault margin.
        let deficit = vault_user_state.margin.checked_neg()?;
        let forfeited = user_pnl.min(deficit);
        let credit = user_pnl.checked_sub(forfeited)?;

        vault_user_state.margin.checked_add_assign(forfeited)?;

        if credit.is_non_zero() {
            user_state.margin.checked_add_assign(credit)?;
        }
    }

    // If PnL is negative or zero, don't penalize the user further.
    // No deficit reduction (need another ADL target).

    Ok(())
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
            perps::{PairId, PairParam, PairState, Param, Position, State},
        },
        grug::{Addr, Coins, MockContext, Storage, Timestamp, Udec128, hash_map},
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
            max_abs_oi: Quantity::new_int(1_000_000),
            ..Default::default()
        }
    }

    fn eth_pair_param() -> PairParam {
        PairParam {
            initial_margin_ratio: Dimensionless::new_permille(100),
            maintenance_margin_ratio: Dimensionless::new_permille(50),
            max_abs_oi: Quantity::new_int(1_000_000),
            ..Default::default()
        }
    }

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

    /// Save vault `UserState` with the given margin.
    fn save_vault_margin(storage: &mut dyn Storage, vault_margin: i128) {
        let vault_state = UserState {
            margin: UsdValue::new_int(vault_margin),
            ..Default::default()
        };
        USER_STATES.save(storage, CONTRACT, &vault_state).unwrap();
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

    // ======================== Public function tests ===========================

    #[test]
    fn no_deficit_errors() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_sender(Addr::mock(99))
            .with_funds(Coins::default());

        let param = default_param();

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            PairState::default(),
        )]);

        // Vault margin = 0 (no deficit).
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

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            PairState::default(),
        )]);

        save_vault_margin(&mut ctx.storage, -5_000);

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
        let pair_state = PairState::default();

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            pair_state.clone(),
        )]);

        let mut vault_user_state = UserState {
            margin: UsdValue::new_int(-5_000),
            ..Default::default()
        };

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
        });

        user_state.margin = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut oracle_querier,
            &mut vault_user_state,
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
        // Deficit = $5,000
        let mut vault_user_state = UserState {
            margin: UsdValue::new_int(-5_000),
            ..Default::default()
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
        });

        user_state.margin = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut oracle_querier,
            &mut vault_user_state,
        );

        assert!(result.is_ok(), "deleverage failed: {:?}", result.err());

        // Position should be removed.
        assert!(
            user_state.positions.is_empty(),
            "user should have no positions after ADL"
        );

        // OI should be reduced.
        assert_eq!(pair_states[&pair_btc()].long_oi, Quantity::ZERO);

        // PnL = $15,000, deficit = |vault margin| = $5,000
        // forfeited = min(15,000, 5,000) = 5,000
        // credit = 15,000 - 5,000 = 10,000
        // vault margin = -5,000 + 5,000 = 0
        assert_eq!(vault_user_state.margin, UsdValue::ZERO);

        // User margin should increase by 10,000.
        assert_eq!(user_state.margin, UsdValue::new_int(20_000));
    }

    #[test]
    fn single_profitable_long_deficit_greater_than_pnl() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        // Deficit = $20,000, PnL will be $15,000
        let mut vault_user_state = UserState {
            margin: UsdValue::new_int(-20_000),
            ..Default::default()
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
        });

        user_state.margin = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut oracle_querier,
            &mut vault_user_state,
        );

        assert!(result.is_ok(), "deleverage failed: {:?}", result.err());

        // All PnL forfeited, no credit to user margin.
        assert_eq!(user_state.margin, UsdValue::new_int(10_000));

        // vault margin = -20,000 + 15,000 = -5,000
        assert_eq!(vault_user_state.margin, UsdValue::new_int(-5_000));
    }

    #[test]
    fn mixed_positions_only_profitable_closed() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let mut vault_user_state = UserState {
            margin: UsdValue::new_int(-3_000),
            ..Default::default()
        };

        let btc_state = PairState::default();
        let eth_state = PairState::default();

        setup_storage(&mut ctx.storage, &param, &[
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
        });

        user_state.margin = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut oracle_querier,
            &mut vault_user_state,
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
    fn margin_credited_not_collected() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_funds(Coins::default());

        let param = default_param();
        let mut vault_user_state = UserState {
            margin: UsdValue::new_int(-5_000),
            ..Default::default()
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
        });

        user_state.margin = UsdValue::new_int(10_000);

        let result = _deleverage(
            &ctx.storage,
            USER,
            &mut pair_states,
            &mut user_state,
            &oracle_prices,
            &mut oracle_querier,
            &mut vault_user_state,
        );

        assert!(result.is_ok(), "deleverage failed: {:?}", result.err());

        // _deleverage never force-collects from user. The non-forfeited PnL
        // is credited to user_state.margin. Original margin is preserved.
        assert!(user_state.margin >= UsdValue::new_int(10_000));
    }

    #[test]
    fn vault_adl_rejected() {
        let mut ctx = MockContext::new()
            .with_contract(CONTRACT)
            .with_sender(Addr::mock(99))
            .with_funds(Coins::default());

        let param = default_param();

        setup_storage(&mut ctx.storage, &param, &[(
            pair_btc(),
            btc_pair_param(),
            PairState::default(),
        )]);

        save_vault_margin(&mut ctx.storage, -5_000);

        let result = super::deleverage(ctx.as_mutable(), CONTRACT);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot ADL the vault"),
            "expected 'cannot ADL the vault' error"
        );
    }
}
