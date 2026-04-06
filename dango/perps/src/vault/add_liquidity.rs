use {
    crate::{
        VIRTUAL_ASSETS, VIRTUAL_SHARES,
        core::compute_user_equity,
        oracle,
        querier::NoCachePerpQuerier,
        state::{STATE, USER_STATES},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{UsdValue, perps::LiquidityAdded},
    grug::{IsZero, MultiplyRatio, MutableCtx, Number as _, Response, Signed, Uint128},
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
    ensure!(ctx.funds.is_empty(), "no funds expected");

    let mut state = STATE.load(ctx.storage)?;

    let mut vault_user_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);

    let mut oracle_querier = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier);

    // ----------------------- Step 1. Validate deposit ------------------------

    ensure!(
        amount.is_positive(),
        "amount of margin to add must be positive"
    );

    ensure!(
        user_state.margin >= amount,
        "insufficient margin: {} (available) < {} (requested to be added)",
        user_state.margin,
        amount
    );

    // --------------------- Step 2. Compute vault equity ----------------------

    let vault_equity = compute_user_equity(&mut oracle_querier, &perp_querier, &vault_user_state)?;

    // Add virtual shares to the current vault share supply to arrive at the
    // effective supply.
    let effective_supply = state.vault_share_supply.checked_add(VIRTUAL_SHARES)?;

    // Add virtual asset to vault equity to arrive at the effective equity.
    let effective_equity = vault_equity.checked_add(VIRTUAL_ASSETS)?;

    ensure!(
        effective_equity.is_positive(),
        "vault is in catastrophic loss! deposit disabled. effective equity: {effective_equity}"
    );

    // -------------------------- Step 3. Mint shares --------------------------

    // Multiply first, then divide, to avoid precision loss from intermediate
    // rounding in Dec128_6 (6 decimal places). The scale factors cancel since
    // both amount and effective_equity are UsdValue (same Dec128_6 scale).
    let shares_to_mint = effective_supply.checked_multiply_ratio_floor(
        amount.into_inner().checked_into_unsigned()?.0,
        effective_equity.into_inner().checked_into_unsigned()?.0,
    )?;

    ensure!(
        shares_to_mint.is_non_zero(),
        "amount of vault shares to be minted is zero"
    );

    if let Some(min_shares_to_mint) = min_shares_to_mint {
        ensure!(
            shares_to_mint >= min_shares_to_mint,
            "too few shares minted: {shares_to_mint} (actual) < {min_shares_to_mint} (expected)"
        );
    }

    // Deduct margin from user and credit to vault.
    user_state.margin.checked_sub_assign(amount)?;
    vault_user_state.margin.checked_add_assign(amount)?;
    state
        .vault_share_supply
        .checked_add_assign(shares_to_mint)?;

    // Update user state.
    user_state.vault_shares.checked_add_assign(shares_to_mint)?;

    // Save all state changes to storage.
    STATE.save(ctx.storage, &state)?;
    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    USER_STATES.save(ctx.storage, ctx.contract, &vault_user_state)?;

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            user = %ctx.sender,
            %amount,
            shares_minted = %shares_to_mint,
            "Liquidity added"
        );
    }

    Ok(Response::new().add_event(LiquidityAdded {
        user: ctx.sender,
        amount,
        shares_minted: shares_to_mint,
    })?)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::state::{STATE, USER_STATES},
        dango_types::{
            UsdValue,
            config::AppConfig,
            perps::{State, UserState},
        },
        grug::{Addr, Coins, MockContext, MockQuerier, ResultExt, Uint128},
    };

    use super::add_liquidity;

    const CONTRACT: Addr = Addr::mock(0);
    const USER: Addr = Addr::mock(1);

    fn mock_querier() -> MockQuerier {
        MockQuerier::new()
            .with_app_config(AppConfig::default())
            .unwrap()
    }

    /// Pre-populate storage with STATE and USER_STATES for the vault and user.
    fn setup(
        storage: &mut dyn grug::Storage,
        vault_share_supply: u128,
        user_margin: UsdValue,
        user_vault_shares: u128,
        vault_margin: UsdValue,
    ) {
        STATE
            .save(storage, &State {
                vault_share_supply: Uint128::new(vault_share_supply),
                ..Default::default()
            })
            .unwrap();

        if !user_margin.is_zero() || user_vault_shares > 0 {
            USER_STATES
                .save(storage, USER, &UserState {
                    margin: user_margin,
                    vault_shares: Uint128::new(user_vault_shares),
                    ..Default::default()
                })
                .unwrap();
        }

        if !vault_margin.is_zero() {
            USER_STATES
                .save(storage, CONTRACT, &UserState {
                    margin: vault_margin,
                    ..Default::default()
                })
                .unwrap();
        }
    }

    // ---- Test 1: first deposit into an empty vault (no pairs) ----
    #[test]
    fn first_deposit_empty_vault() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(&mut ctx.storage, 0, UsdValue::new_int(1), 0, UsdValue::ZERO);

        add_liquidity(ctx.as_mutable(), UsdValue::new_int(1), None).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(user_state.vault_shares, Uint128::new(1_000_000));
        assert_eq!(user_state.margin, UsdValue::ZERO);
    }

    // ---- Test 2: second deposit of same size into a non-empty vault ----
    #[test]
    fn second_deposit_same_size() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(
            &mut ctx.storage,
            1_000_000,
            UsdValue::new_int(1),
            0,
            UsdValue::new_int(1),
        );

        add_liquidity(ctx.as_mutable(), UsdValue::new_int(1), None).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(user_state.vault_shares, Uint128::new(1_000_000));
        assert_eq!(user_state.margin, UsdValue::ZERO);
    }

    // ---- Test 3: zero deposit ----
    #[test]
    fn zero_add_liquidity() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(&mut ctx.storage, 0, UsdValue::ZERO, 0, UsdValue::ZERO);

        add_liquidity(ctx.as_mutable(), UsdValue::ZERO, None)
            .should_fail_with_error("amount of margin to add must be positive");
    }

    // ---- Test 4: insufficient margin rejected ----
    #[test]
    fn insufficient_margin_rejected() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(
            &mut ctx.storage,
            0,
            UsdValue::new_permille(500),
            0,
            UsdValue::ZERO,
        );

        add_liquidity(ctx.as_mutable(), UsdValue::new_int(1), None)
            .should_fail_with_error("insufficient margin");
    }

    // ---- Test 6: min_shares passes ----
    #[test]
    fn min_shares_passes() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(&mut ctx.storage, 0, UsdValue::new_int(1), 0, UsdValue::ZERO);

        add_liquidity(
            ctx.as_mutable(),
            UsdValue::new_int(1),
            Some(Uint128::new(1_000_000)),
        )
        .should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(user_state.vault_shares, Uint128::new(1_000_000));
    }

    // ---- Test 7: min_shares fails ----
    #[test]
    fn min_shares_fails() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(&mut ctx.storage, 0, UsdValue::new_int(1), 0, UsdValue::ZERO);

        add_liquidity(
            ctx.as_mutable(),
            UsdValue::new_int(1),
            Some(Uint128::new(1_000_001)),
        )
        .should_fail_with_error("too few shares minted");
    }

    // ---- Test 11: large deposit no overflow ----
    #[test]
    fn large_deposit_no_overflow() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        let one_billion = 1_000_000_000_i128;

        setup(
            &mut ctx.storage,
            one_billion as u128 * 1_000_000,
            UsdValue::new_int(one_billion),
            0,
            UsdValue::new_int(one_billion),
        );

        add_liquidity(ctx.as_mutable(), UsdValue::new_int(one_billion), None).should_succeed();

        // With existing margin equal to deposit, shares should be close to supply
        // (slightly less due to virtual shares/assets dilution).
        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert!(user_state.vault_shares > Uint128::new(0));
    }

    // ---- Test: small deposit should not lose precision ----
    #[test]
    fn small_deposit_precision() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        // Vault: $2 margin, 2M shares.
        // effective_supply = 2M + 1M = 3M
        // effective_equity = $2 + $1 = $3 (raw = 3_000_000)
        //
        // Deposit $0.000001 (raw = 1).
        // Correct: floor(3_000_000 * 1 / 3_000_000) = 1 share.
        // Buggy (divide-first): ratio = 1 * 10^6 / 3_000_000 = 0 → zero shares.
        setup(
            &mut ctx.storage,
            2_000_000,
            UsdValue::new_raw(1),
            0,
            UsdValue::new_int(2),
        );

        add_liquidity(ctx.as_mutable(), UsdValue::new_raw(1), None).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(user_state.vault_shares, Uint128::new(1));
    }

    // ---- Test 13: exact division yields exact shares ----
    #[test]
    fn exact_division_shares() {
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        // effective_supply = 3M, effective_equity = $3
        // floor(3M * 1M / 3M) = 1_000_000 (exact, no rounding needed).
        setup(
            &mut ctx.storage,
            2_000_000,
            UsdValue::new_int(1),
            0,
            UsdValue::new_int(2),
        );

        add_liquidity(ctx.as_mutable(), UsdValue::new_int(1), None).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        assert_eq!(user_state.vault_shares, Uint128::new(1_000_000));
    }
}
