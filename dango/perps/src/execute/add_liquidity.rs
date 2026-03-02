use {
    crate::{
        NoCachePerpQuerier, STATE, USER_STATES,
        core::compute_user_equity,
        execute::{ORACLE, VIRTUAL_ASSETS, VIRTUAL_SHARES},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{UsdValue, perps::UserState},
    grug::{IsZero, MultiplyFraction, MutableCtx, Number as _, Response, Signed, Uint128},
};

/// Add liquidity to the counterparty vault by transferring margin to the vault.
///
/// Mutates: `STATE` (vault_share_supply), `USER_STATES` (margin, vault_shares).
///
/// Returns: `Response`.
pub fn add_liquidity(
    ctx: MutableCtx,
    amount: UsdValue,
    min_shares_to_mint: Option<Uint128>,
) -> anyhow::Result<Response> {
    // ---------------------------- 1. Preparation -----------------------------

    ensure!(ctx.funds.is_empty(), "no funds expected");

    let mut state = STATE.load(ctx.storage)?;

    let mut vault_user_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    ensure!(
        !vault_user_state.margin.is_negative(),
        "deposits paused: unresolved ADL deficit"
    );

    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // --------------------------- 2. Business logic ---------------------------

    _add_liquidity(
        &perp_querier,
        &mut oracle_querier,
        &mut state.vault_share_supply,
        &mut user_state,
        &mut vault_user_state,
        amount,
        min_shares_to_mint,
    )?;

    // ------------------------ 3. Apply state changes -------------------------

    STATE.save(ctx.storage, &state)?;
    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    USER_STATES.save(ctx.storage, ctx.contract, &vault_user_state)?;

    Ok(Response::new())
}

/// The actual logic for handling the add-liquidity operation.
///
/// Mutates:
/// - `vault_share_supply`
/// - `user_state` (margin, vault_shares)
/// - `vault_user_state` (margin)
///
/// Returns: `()` on success.
fn _add_liquidity(
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
    vault_share_supply: &mut Uint128,
    user_state: &mut UserState,
    vault_user_state: &mut UserState,
    amount: UsdValue,
    min_shares_to_mint: Option<Uint128>,
) -> anyhow::Result<()> {
    // ----------------------- Step 1. Validate deposit ------------------------

    ensure!(amount.is_positive(), "nothing to do");

    ensure!(user_state.margin >= amount, "insufficient margin");

    // --------------------- Step 2. Compute vault equity ----------------------

    let vault_equity = compute_user_equity(
        vault_user_state.margin,
        vault_user_state,
        perp_querier,
        oracle_querier,
    )?;

    // Add virtual shares to the current vault share supply to arrive at the
    // effective supply.
    let effective_supply = vault_share_supply.checked_add(VIRTUAL_SHARES)?;

    // Add virtual asset to vault equity to arrive at the effective equity.
    let effective_equity = vault_equity.checked_add(VIRTUAL_ASSETS)?;

    ensure!(
        effective_equity.is_positive(),
        "vault is in catastrophic loss! deposit disabled. effective equity: {effective_equity}"
    );

    // -------------------------- Step 3. Mint shares --------------------------

    // deposit_margin is already a UsdValue — no conversion needed.
    let ratio = amount
        .checked_div(effective_equity)?
        .into_inner()
        .checked_into_unsigned()?;
    let shares_to_mint = effective_supply.checked_mul_dec_floor(ratio)?;

    ensure!(shares_to_mint.is_non_zero(), "nothing to do");

    if let Some(min_shares_to_mint) = min_shares_to_mint {
        ensure!(
            shares_to_mint >= min_shares_to_mint,
            "too few shares minted: {shares_to_mint} (actual) < {min_shares_to_mint} (expected)"
        );
    }

    // Deduct margin from user and credit to vault.
    user_state.margin.checked_sub_assign(amount)?;
    vault_user_state.margin.checked_add_assign(amount)?;
    vault_share_supply.checked_add_assign(shares_to_mint)?;

    // Update user state.
    user_state.vault_shares.checked_add_assign(shares_to_mint)?;

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::perps::UserState,
        grug::{MockStorage, NumberConst, Uint128, hash_map},
    };

    // ---- Test 1: first deposit into an empty vault (no pairs) ----
    #[test]
    fn first_deposit_empty_vault() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::ZERO;
        let mut user_state = UserState {
            margin: UsdValue::new_int(1),
            ..Default::default()
        };
        let mut vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _add_liquidity(
            &perp_querier,
            &mut oracle_querier,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            UsdValue::new_int(1),
            None,
        )
        .unwrap();

        assert_eq!(user_state.vault_shares, Uint128::new(1_000_000));
        assert_eq!(user_state.margin, UsdValue::ZERO);
    }

    // ---- Test 2: second deposit of same size into a non-empty vault ----
    #[test]
    fn second_deposit_same_size() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::new(1_000_000);
        let mut user_state = UserState {
            margin: UsdValue::new_int(1),
            ..Default::default()
        };
        let mut vault_user_state = UserState {
            margin: UsdValue::new_int(1),
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _add_liquidity(
            &perp_querier,
            &mut oracle_querier,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            UsdValue::new_int(1),
            None,
        )
        .unwrap();

        assert_eq!(user_state.vault_shares, Uint128::new(1_000_000));
        assert_eq!(user_state.margin, UsdValue::ZERO);
    }

    // ---- Test 3: zero deposit ----
    #[test]
    fn zero_add_liquidity() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::ZERO;
        let mut user_state = UserState::default();
        let mut vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let err = _add_liquidity(
            &perp_querier,
            &mut oracle_querier,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            UsdValue::ZERO,
            None,
        )
        .unwrap_err();

        assert!(err.to_string().contains("nothing to do"));
    }

    // ---- Test 4: insufficient margin rejected ----
    #[test]
    fn insufficient_margin_rejected() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::ZERO;
        let mut user_state = UserState {
            margin: UsdValue::new_permille(500),
            ..Default::default()
        };
        let mut vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let err = _add_liquidity(
            &perp_querier,
            &mut oracle_querier,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            UsdValue::new_int(1),
            None,
        )
        .unwrap_err();

        assert!(err.to_string().contains("insufficient margin"));
    }

    // ---- Test 6: min_shares passes ----
    #[test]
    fn min_shares_passes() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::ZERO;
        let mut user_state = UserState {
            margin: UsdValue::new_int(1),
            ..Default::default()
        };
        let mut vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _add_liquidity(
            &perp_querier,
            &mut oracle_querier,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            UsdValue::new_int(1),
            Some(Uint128::new(1_000_000)),
        )
        .unwrap();

        assert_eq!(user_state.vault_shares, Uint128::new(1_000_000));
    }

    // ---- Test 7: min_shares fails ----
    #[test]
    fn min_shares_fails() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::ZERO;
        let mut user_state = UserState {
            margin: UsdValue::new_int(1),
            ..Default::default()
        };
        let mut vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let err = _add_liquidity(
            &perp_querier,
            &mut oracle_querier,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            UsdValue::new_int(1),
            Some(Uint128::new(1_000_001)),
        )
        .unwrap_err();

        assert!(err.to_string().contains("too few shares minted"));
    }

    // ---- Test 11: large deposit no overflow ----
    #[test]
    fn large_deposit_no_overflow() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let one_billion = 1_000_000_000_u128;
        let one_billion_shares: u128 = one_billion * 1_000_000;

        let mut vault_share_supply = Uint128::new(one_billion_shares);
        let mut user_state = UserState {
            margin: UsdValue::new_int(one_billion as i128),
            ..Default::default()
        };
        let mut vault_user_state = UserState {
            margin: UsdValue::new_int(one_billion as i128),
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _add_liquidity(
            &perp_querier,
            &mut oracle_querier,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            UsdValue::new_int(one_billion as i128),
            None,
        )
        .unwrap();

        // With existing margin equal to deposit, shares should be close to supply
        // (slightly less due to virtual shares/assets dilution).
        assert!(user_state.vault_shares > Uint128::new(0));
    }

    // ---- Test 13: shares rounded floor ----
    #[test]
    fn shares_rounded_floor() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {});

        let mut vault_share_supply = Uint128::new(2_000_000);
        let mut user_state = UserState {
            margin: UsdValue::new_int(1),
            ..Default::default()
        };
        let mut vault_user_state = UserState {
            margin: UsdValue::new_int(2),
            ..Default::default()
        };
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        _add_liquidity(
            &perp_querier,
            &mut oracle_querier,
            &mut vault_share_supply,
            &mut user_state,
            &mut vault_user_state,
            UsdValue::new_int(1),
            None,
        )
        .unwrap();

        // Should be 999_999, not 1_000_000 (ceil) or 1_000_002 (if division ceiled).
        assert_eq!(user_state.vault_shares, Uint128::new(999_999));
    }
}
