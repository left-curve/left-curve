use {
    crate::{
        MAX_ORACLE_STALENESS, VIRTUAL_ASSETS, VIRTUAL_SHARES,
        core::{compute_available_margin, compute_user_equity},
        oracle,
        querier::NoCachePerpQuerier,
        state::{PARAM, STATE, USER_STATES},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_order_book::UsdValue,
    dango_types::perps::{LiquidityUnlocking, Param, State, Unlock, UserState},
    grug::{
        Dec128_6, Int128, IsZero, MultiplyRatio, MutableCtx, Number as _, Response, Timestamp,
        Uint128,
    },
};

/// Request to withdraw liquidity from the counterparty vault.
/// Records a `UsdValue` unlock that will be converted to tokens at claim time.
pub fn remove_liquidity(ctx: MutableCtx, shares_to_burn: Uint128) -> anyhow::Result<Response> {
    ensure!(ctx.funds.is_empty(), "unexpected funds: {:?}", ctx.funds);

    // ---------------------------- 1. Preparation -----------------------------

    let param = PARAM.load(ctx.storage)?;

    let mut state = STATE.load(ctx.storage)?;

    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let mut vault_user_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);

    let mut oracle_querier = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier)
        .with_no_older_than(ctx.block.timestamp - MAX_ORACLE_STALENESS);

    // --------------------------- 2. Business logic ---------------------------

    let (amount, end_time) = _remove_liquidity(
        ctx.block.timestamp,
        &perp_querier,
        &mut oracle_querier,
        &param,
        &mut state,
        &mut user_state,
        &mut vault_user_state,
        shares_to_burn,
    )?;

    // ------------------------ 3. Apply state changes -------------------------

    STATE.save(ctx.storage, &state)?;
    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    USER_STATES.save(ctx.storage, ctx.contract, &vault_user_state)?;

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            user = %ctx.sender,
            %shares_to_burn,
            %amount,
            "Liquidity removal queued"
        );
    }

    #[cfg(feature = "metrics")]
    {
        metrics::histogram!(crate::metrics::LABEL_VAULT_WITHDRAWAL_AMOUNT).record(amount.to_f64());
    }

    Ok(Response::new().add_event(LiquidityUnlocking {
        user: ctx.sender,
        amount,
        shares_burned: shares_to_burn,
        end_time,
    })?)
}

/// The actual logic for handling the remove-liquidity operation.
///
/// Mutates:
///
/// - `state` (vault_share_supply)
/// - `user_state` (vault_shares, unlock queue)
/// - `vault_user_state` (margin)
///
/// Returns: `(amount_to_release, end_time)`.
fn _remove_liquidity(
    current_time: Timestamp,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
    param: &Param,
    state: &mut State,
    user_state: &mut UserState,
    vault_user_state: &mut UserState,
    shares_to_burn: Uint128,
) -> anyhow::Result<(UsdValue, Timestamp)> {
    // -------------------- Step 1. Validate shares to burn --------------------

    ensure!(
        shares_to_burn.is_non_zero(),
        "amount of shares to burn must be positive"
    );

    ensure!(
        user_state.vault_shares >= shares_to_burn,
        "insufficient vault shares: {} (available) < {} (requested to burn)",
        user_state.vault_shares,
        shares_to_burn
    );

    ensure!(
        user_state.unlocks.len() < param.max_unlocks,
        "too many pending unlocks! current: {}, max: {}",
        user_state.unlocks.len(),
        param.max_unlocks
    );

    // --------------------- Step 2. Compute vault equity ----------------------

    let vault_equity = compute_user_equity(oracle_querier, perp_querier, vault_user_state)?;

    let effective_supply = state.vault_share_supply.checked_add(VIRTUAL_SHARES)?;

    let effective_equity = vault_equity.checked_add(VIRTUAL_ASSETS)?;

    ensure!(
        effective_equity.is_positive(),
        "vault is in catastrophic loss! withdrawal disabled. effective equity: {effective_equity}"
    );

    // ------------------- Step 3. Compute amount to release -------------------

    // Multiply first, then divide, to avoid precision loss from intermediate
    // rounding in Dec128_6 (6 decimal places).
    let amount_to_release = {
        let raw = effective_equity
            .into_inner()
            .0
            .checked_multiply_ratio_floor(
                Int128::new(i128::try_from(shares_to_burn.0)?),
                Int128::new(i128::try_from(effective_supply.0)?),
            )?;
        UsdValue::new(Dec128_6::raw(raw))
    };

    // ------------------------- Step 4. Margin check --------------------------

    let vault_available_margin =
        compute_available_margin(oracle_querier, perp_querier, vault_user_state)?;

    ensure!(
        vault_available_margin >= amount_to_release,
        "insufficient vault available margin to cover withdrawal: {} (available) < {} (release)",
        vault_available_margin,
        amount_to_release
    );

    // ---------------------- Step 5. Schedule the unlock ----------------------

    let end_time = current_time + param.vault_cooldown_period;
    let unlock = Unlock {
        amount_to_release,
        end_time,
    };

    // Reduce vault share supply.
    state
        .vault_share_supply
        .checked_sub_assign(shares_to_burn)?;

    // Burn the vault shares from the user's state.
    user_state.vault_shares.checked_sub_assign(shares_to_burn)?;

    // Create an unlock in the user's state.
    user_state.unlocks.push_back(unlock);

    // Deduct margin from the vault.
    vault_user_state
        .margin
        .checked_sub_assign(amount_to_release)?;

    Ok((amount_to_release, end_time))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::UsdValue,
        grug::{Duration, MockStorage, Uint128, hash_map},
        std::collections::VecDeque,
    };

    fn default_param() -> Param {
        Param {
            vault_cooldown_period: Duration::from_seconds(86400), // 1 day
            max_unlocks: 10,
            ..Default::default()
        }
    }

    fn state_with_supply(supply: u128) -> State {
        State {
            vault_share_supply: Uint128::new(supply),
            ..Default::default()
        }
    }

    /// Helper: create a vault UserState with the given margin.
    fn vault_state_with_margin(margin: i128) -> UserState {
        UserState {
            margin: UsdValue::new_int(margin),
            ..Default::default()
        }
    }

    // ---- Test 1: first withdrawal symmetric with deposit ----
    // Deposit 1 USDC ($1) → 1M shares, withdraw all 1M shares → $1 unlock.
    // vault margin=$1, supply=1_000_000
    // effective_supply = 1M + 1M = 2M
    // vault_equity = $1, effective_equity = $2
    // amount_to_release = floor($2 * 1_000_000 / 2_000_000) = $1
    #[test]
    fn first_withdrawal_symmetric() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});
        let param = default_param();
        let mut state = state_with_supply(1_000_000);
        let mut user_state = UserState {
            vault_shares: Uint128::new(1_000_000),
            ..Default::default()
        };
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _remove_liquidity(
            Timestamp::from_seconds(0),
            &perp_querier,
            &mut oracle_querier,
            &param,
            &mut state,
            &mut user_state,
            &mut vault_user_state,
            Uint128::new(1_000_000),
        )
        .unwrap();

        let unlock = user_state.unlocks.back().unwrap();
        assert_eq!(unlock.amount_to_release, UsdValue::new_int(1));
    }

    // ---- Test 2: partial withdrawal ----
    #[test]
    fn partial_withdrawal() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});
        let param = default_param();
        let mut state = state_with_supply(1_000_000);
        let mut user_state = UserState {
            vault_shares: Uint128::new(500_000),
            ..Default::default()
        };
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _remove_liquidity(
            Timestamp::from_seconds(0),
            &perp_querier,
            &mut oracle_querier,
            &param,
            &mut state,
            &mut user_state,
            &mut vault_user_state,
            Uint128::new(500_000),
        )
        .unwrap();

        let unlock = user_state.unlocks.back().unwrap();
        assert!(unlock.amount_to_release > UsdValue::ZERO);
    }

    // ---- Test 3: zero shares rejected ----
    #[test]
    fn zero_shares_rejected() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});
        let param = default_param();
        let mut state = state_with_supply(1_000_000);
        let mut user_state = UserState::default();
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let err = _remove_liquidity(
            Timestamp::from_seconds(0),
            &perp_querier,
            &mut oracle_querier,
            &param,
            &mut state,
            &mut user_state,
            &mut vault_user_state,
            Uint128::new(0),
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("amount of shares to burn must be positive")
        );
    }

    // ---- Test 7: max unlocks exceeded ----
    #[test]
    fn max_unlocks_exceeded() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});
        let param = Param {
            max_unlocks: 2,
            vault_cooldown_period: Duration::from_seconds(86400),
            ..Default::default()
        };
        let mut state = state_with_supply(1_000_000);
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        // Already at max_unlocks (2 existing unlocks).
        let mut user_state = UserState {
            vault_shares: Uint128::new(500_000),
            unlocks: VecDeque::from([
                Unlock {
                    amount_to_release: UsdValue::new_int(100),
                    end_time: Timestamp::from_seconds(100),
                },
                Unlock {
                    amount_to_release: UsdValue::new_int(200),
                    end_time: Timestamp::from_seconds(200),
                },
            ]),
            ..Default::default()
        };

        let err = _remove_liquidity(
            Timestamp::from_seconds(0),
            &perp_querier,
            &mut oracle_querier,
            &param,
            &mut state,
            &mut user_state,
            &mut vault_user_state,
            Uint128::new(500_000),
        )
        .unwrap_err();

        assert!(err.to_string().contains("too many"));
    }

    // ---- Test: small burn should not lose precision ----
    #[test]
    fn small_burn_precision() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});
        let param = default_param();

        // Vault: $2 margin, 2M shares.
        // effective_supply = 2M + 1M = 3M
        // effective_equity = $2 + $1 = $3 (raw = 3_000_000)
        //
        // Burn 1 share.
        // Correct: floor(3_000_000 * 1 / 3_000_000) = 1 raw = $0.000001.
        // Buggy (divide-first): ratio = 1 * 10^6 / 3_000_000 = 0 → amount = $0.
        let mut state = state_with_supply(2_000_000);
        let mut user_state = UserState {
            vault_shares: Uint128::new(1),
            ..Default::default()
        };
        let mut vault_user_state = vault_state_with_margin(2);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let (amount, _end_time) = _remove_liquidity(
            Timestamp::from_seconds(0),
            &perp_querier,
            &mut oracle_querier,
            &param,
            &mut state,
            &mut user_state,
            &mut vault_user_state,
            Uint128::new(1),
        )
        .unwrap();

        assert_eq!(amount, UsdValue::new_raw(1));
    }

    // ---- Test 12: unlock end_time is correct ----
    #[test]
    fn unlock_end_time_correct() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});
        let param = Param {
            vault_cooldown_period: Duration::from_seconds(172_800), // 2 days
            max_unlocks: 10,
            ..Default::default()
        };
        let mut state = state_with_supply(1_000_000);
        let mut user_state = UserState {
            vault_shares: Uint128::new(500_000),
            ..Default::default()
        };
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _remove_liquidity(
            Timestamp::from_seconds(1_000_000),
            &perp_querier,
            &mut oracle_querier,
            &param,
            &mut state,
            &mut user_state,
            &mut vault_user_state,
            Uint128::new(500_000),
        )
        .unwrap();

        let unlock = user_state.unlocks.back().unwrap();
        assert_eq!(unlock.end_time, Timestamp::from_seconds(1_172_800));
    }
}
