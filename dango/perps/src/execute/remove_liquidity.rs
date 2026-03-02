use {
    crate::{
        NoCachePerpQuerier, PARAM, STATE, USER_STATES,
        core::compute_user_equity,
        execute::{ORACLE, VIRTUAL_ASSETS, VIRTUAL_SHARES},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless,
        perps::{Param, Unlock, UserState},
    },
    grug::{IsZero, MutableCtx, Number as _, Response, Timestamp, Uint128},
};

/// Request to withdraw liquidity from the counterparty vault.
/// Records a `UsdValue` unlock that will be converted to tokens at claim time.
pub fn remove_liquidity(ctx: MutableCtx, shares_to_burn: Uint128) -> anyhow::Result<Response> {
    ensure!(ctx.funds.is_empty(), "unexpected funds: {:?}", ctx.funds);

    // ---------------------------- 1. Preparation -----------------------------

    let param = PARAM.load(ctx.storage)?;

    let mut state = STATE.load(ctx.storage)?;

    let mut vault_user_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    ensure!(
        !vault_user_state.margin.is_negative(),
        "withdrawals paused: unresolved ADL deficit"
    );

    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // --------------------------- 2. Business logic ---------------------------

    _remove_liquidity(
        ctx.block.timestamp,
        shares_to_burn,
        &param,
        &mut state.vault_share_supply,
        &mut user_state,
        &mut vault_user_state,
        &perp_querier,
        &mut oracle_querier,
    )?;

    // ------------------------ 3. Apply state changes -------------------------

    STATE.save(ctx.storage, &state)?;
    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    USER_STATES.save(ctx.storage, ctx.contract, &vault_user_state)?;

    Ok(Response::new())
}

/// The actual logic for handling the remove-liquidity operation.
///
/// Mutates:
///
/// - `vault_share_supply`
/// - `user_state` (vault_shares, unlock queue)
/// - `vault_user_state` (margin)
fn _remove_liquidity(
    current_time: Timestamp,
    shares_to_burn: Uint128,
    param: &Param,
    vault_share_supply: &mut Uint128,
    user_state: &mut UserState,
    vault_user_state: &mut UserState,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<()> {
    // -------------------- Step 1. Validate shares to burn --------------------

    ensure!(shares_to_burn.is_non_zero(), "nothing to do");

    ensure!(
        user_state.vault_shares >= shares_to_burn,
        "insufficient vault shares"
    );

    ensure!(
        user_state.unlocks.len() < param.max_unlocks,
        "too many pending unlocks"
    );

    // --------------------- Step 2. Compute vault equity ----------------------

    let vault_equity = compute_user_equity(vault_user_state, perp_querier, oracle_querier)?;

    let effective_supply = vault_share_supply.checked_add(VIRTUAL_SHARES)?;

    let effective_equity = vault_equity.checked_add(VIRTUAL_ASSETS)?;

    ensure!(
        effective_equity.is_positive(),
        "vault is in catastrophic loss! withdrawal disabled. effective equity: {effective_equity}"
    );

    // ------------------- Step 3. Compute amount to release -------------------

    // Compute the proportional USD value to release.
    // ratio = shares_to_burn / effective_supply
    let ratio = Dimensionless::new_raw(i128::try_from(shares_to_burn.0)?)
        .checked_div(Dimensionless::new_raw(i128::try_from(effective_supply.0)?))?;

    // amount_to_release = effective_equity * ratio (floor-rounded for safety)
    let amount_to_release = effective_equity.checked_mul(ratio)?;

    ensure!(
        vault_user_state.margin >= amount_to_release,
        "insufficient vault margin to cover withdrawal: {} (margin) < {} (release)",
        vault_user_state.margin,
        amount_to_release
    );

    let end_time = current_time + param.vault_cooldown_period;

    let unlock = Unlock {
        amount_to_release,
        end_time,
    };

    // Update vault state.
    vault_user_state
        .margin
        .checked_sub_assign(amount_to_release)?;
    vault_share_supply.checked_sub_assign(shares_to_burn)?;

    // Update user state.
    user_state.vault_shares.checked_sub_assign(shares_to_burn)?;
    user_state.unlocks.push_back(unlock);

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::UsdValue,
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

        let mut vault_share_supply = Uint128::new(1_000_000);
        let param = default_param();
        let mut user_state = UserState {
            vault_shares: Uint128::new(1_000_000),
            ..Default::default()
        };
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _remove_liquidity(
            Timestamp::from_seconds(0),
            Uint128::new(1_000_000),
            &param,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            &perp_querier,
            &mut oracle_querier,
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

        let mut vault_share_supply = Uint128::new(1_000_000);
        let param = default_param();
        let mut user_state = UserState {
            vault_shares: Uint128::new(500_000),
            ..Default::default()
        };
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _remove_liquidity(
            Timestamp::from_seconds(0),
            Uint128::new(500_000),
            &param,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            &perp_querier,
            &mut oracle_querier,
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

        let mut vault_share_supply = Uint128::new(1_000_000);
        let param = default_param();
        let mut user_state = UserState::default();
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let err = _remove_liquidity(
            Timestamp::from_seconds(0),
            Uint128::new(0),
            &param,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("nothing to do"));
    }

    // ---- Test 7: max unlocks exceeded ----
    #[test]
    fn max_unlocks_exceeded() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::new(1_000_000);
        let param = Param {
            max_unlocks: 2,
            vault_cooldown_period: Duration::from_seconds(86400),
            ..Default::default()
        };
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
            Uint128::new(500_000),
            &param,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("too many"));
    }

    // ---- Test 12: unlock end_time is correct ----
    #[test]
    fn unlock_end_time_correct() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::new(1_000_000);
        let param = Param {
            vault_cooldown_period: Duration::from_seconds(172_800), // 2 days
            max_unlocks: 10,
            ..Default::default()
        };
        let mut user_state = UserState {
            vault_shares: Uint128::new(500_000),
            ..Default::default()
        };
        let mut vault_user_state = vault_state_with_margin(1);
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _remove_liquidity(
            Timestamp::from_seconds(1_000_000),
            Uint128::new(500_000),
            &param,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap();

        let unlock = user_state.unlocks.back().unwrap();
        assert_eq!(unlock.end_time, Timestamp::from_seconds(1_172_800));
    }
}
