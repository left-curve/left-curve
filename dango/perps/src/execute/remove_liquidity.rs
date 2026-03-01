use {
    crate::{
        NoCachePerpQuerier, PARAM, STATE, USER_STATES,
        core::compute_user_equity,
        execute::{BANK, ORACLE, VIRTUAL_ASSETS, VIRTUAL_SHARES},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Dimensionless, bank,
        perps::{self, Param, State, Unlock, UserState},
    },
    grug::{Coins, IsZero, Message, MutableCtx, Number as _, Response, Timestamp, Uint128},
};

/// Request to withdraw liquidity from the counterparty vault.
/// Records a `UsdValue` unlock that will be converted to tokens at claim time.
///
/// Mutates: `STATE` (vault_margin, vault_share_supply), `USER_STATES` (unlock queue).
///
/// Returns: `Response` with a bank burn message.
pub fn remove_liquidity(ctx: MutableCtx) -> anyhow::Result<Response> {
    // ---------------------------- 1. Preparation -----------------------------

    let param = PARAM.load(ctx.storage)?;

    let mut state = STATE.load(ctx.storage)?;

    ensure!(
        state.adl_deficit.is_zero(),
        "withdrawals paused: unresolved ADL deficit"
    );

    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let vault_user_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // --------------------------- 2. Business logic ---------------------------

    let (shares_to_burn, unlock) = _remove_liquidity(
        ctx.block.timestamp,
        ctx.funds,
        &state,
        &param,
        &user_state,
        &vault_user_state,
        &perp_querier,
        &mut oracle_querier,
    )?;

    // Update global state: vault_margin is UsdValue.
    (state.vault_margin).checked_sub_assign(unlock.amount_to_release)?;
    (state.vault_share_supply).checked_sub_assign(shares_to_burn)?;

    // Update user state.
    user_state.unlocks.push_back(unlock);

    // ------------------------ 3. Apply state changes -------------------------

    // Save the updated global and user states.
    STATE.save(ctx.storage, &state)?;
    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;

    // Burn the share tokens that were sent as funds.
    Ok(Response::new().add_message(Message::execute(
        BANK,
        &bank::ExecuteMsg::Burn {
            from: ctx.contract,
            coins: Coins::one(perps::DENOM.clone(), shares_to_burn)?,
        },
        Coins::new(),
    )?))
}

/// The actual logic for handling the remove-liquidity operation.
///
/// Mutates: nothing (pure computation).
///
/// Returns: 1) the amount of shares to burn, 2) the unlock record (with UsdValue).
fn _remove_liquidity(
    current_time: Timestamp,
    mut funds: Coins,
    state: &State,
    param: &Param,
    user_state: &UserState,
    vault_user_state: &UserState,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<(Uint128, Unlock)> {
    // -------------------- Step 1. Extract shares to burn ---------------------

    let shares_to_burn = funds.take(perps::DENOM.clone()).amount;

    ensure!(funds.is_empty(), "unexpected funds: {funds:?}");

    ensure!(shares_to_burn.is_non_zero(), "nothing to do");

    ensure!(
        user_state.unlocks.len() < param.max_unlocks,
        "too many pending unlocks"
    );

    // --------------------- Step 2. Compute vault equity ----------------------

    // vault_margin is already UsdValue — no base→USD conversion needed.
    let vault_equity = compute_user_equity(
        state.vault_margin,
        vault_user_state,
        perp_querier,
        oracle_querier,
    )?;

    let effective_supply = state.vault_share_supply.checked_add(VIRTUAL_SHARES)?;

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
        state.vault_margin >= amount_to_release,
        "insufficient vault margin to cover withdrawal: {} (margin) < {} (release)",
        state.vault_margin,
        amount_to_release
    );

    let end_time = current_time + param.vault_cooldown_period;

    Ok((shares_to_burn, Unlock {
        amount_to_release,
        end_time,
    }))
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::UsdValue,
        grug::{Coin, Duration, MockStorage, Uint128, hash_map},
        std::collections::VecDeque,
    };

    /// Helper: build `Coins` containing `amount` of the vault share token.
    fn share_coins(amount: u128) -> Coins {
        Coins::one(perps::DENOM.clone(), amount).unwrap()
    }

    /// Helper: default param with known cooldown and max_unlocks.
    fn default_param() -> Param {
        Param {
            vault_cooldown_period: Duration::from_seconds(86400), // 1 day
            max_unlocks: 10,
            ..Default::default()
        }
    }

    // ---- Test 1: first withdrawal symmetric with deposit ----
    // Deposit 1 USDC ($1) → 1M shares, withdraw all 1M shares → $1 unlock.
    // state: vault_margin=$1, supply=1_000_000
    // effective_supply = 1M + 1M = 2M
    // vault_equity = $1, effective_equity = $2
    // amount_to_release = floor($2 * 1_000_000 / 2_000_000) = $1
    #[test]
    fn first_withdrawal_symmetric() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let state = State {
            vault_margin: UsdValue::new_int(1),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let (shares, unlock) = _remove_liquidity(
            Timestamp::from_seconds(0),
            share_coins(1_000_000),
            &state,
            &param,
            &user_state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(1_000_000));
        assert_eq!(unlock.amount_to_release, UsdValue::new_int(1));
    }

    // ---- Test 2: partial withdrawal ----
    // Same state, withdraw half the shares → half the equity.
    #[test]
    fn partial_withdrawal() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let state = State {
            vault_margin: UsdValue::new_int(1),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let (_, unlock) = _remove_liquidity(
            Timestamp::from_seconds(0),
            share_coins(500_000),
            &state,
            &param,
            &user_state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap();

        // effective_equity = $2, ratio = 500000/2000000 = 0.25
        // amount = floor($2 * 0.25) = $0.50
        assert!(unlock.amount_to_release > UsdValue::ZERO);
    }

    // ---- Test 3: zero shares rejected ----
    #[test]
    fn zero_shares_rejected() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let state = State {
            vault_margin: UsdValue::new_int(1),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let err = _remove_liquidity(
            Timestamp::from_seconds(0),
            Coins::new(),
            &state,
            &param,
            &user_state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("nothing to do"));
    }

    // ---- Test 4: unexpected coins rejected ----
    #[test]
    fn unexpected_coins_rejected() {
        use dango_types::constants::eth;

        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let state = State {
            vault_margin: UsdValue::new_int(1),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let mut funds = share_coins(1_000_000);
        funds
            .insert(Coin {
                denom: eth::DENOM.clone(),
                amount: Uint128::new(100),
            })
            .unwrap();

        let err = _remove_liquidity(
            Timestamp::from_seconds(0),
            funds,
            &state,
            &param,
            &user_state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("unexpected"));
    }

    // ---- Test 7: max unlocks exceeded ----
    #[test]
    fn max_unlocks_exceeded() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let state = State {
            vault_margin: UsdValue::new_int(1),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = Param {
            max_unlocks: 2,
            vault_cooldown_period: Duration::from_seconds(86400),
            ..Default::default()
        };
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        // Already at max_unlocks (2 existing unlocks).
        let user_state = UserState {
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
            share_coins(500_000),
            &state,
            &param,
            &user_state,
            &vault_user_state,
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

        let state = State {
            vault_margin: UsdValue::new_int(1),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = Param {
            vault_cooldown_period: Duration::from_seconds(172_800), // 2 days
            max_unlocks: 10,
            ..Default::default()
        };
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let (_, unlock) = _remove_liquidity(
            Timestamp::from_seconds(1_000_000),
            share_coins(500_000),
            &state,
            &param,
            &user_state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(unlock.end_time, Timestamp::from_seconds(1_172_800));
    }
}
