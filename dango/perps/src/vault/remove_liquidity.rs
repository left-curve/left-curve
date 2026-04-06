use {
    crate::{
        VIRTUAL_ASSETS, VIRTUAL_SHARES,
        core::compute_user_equity,
        oracle,
        querier::NoCachePerpQuerier,
        state::{PARAM, STATE, USER_STATES},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        UsdValue,
        perps::{LiquidityUnlocking, Unlock},
    },
    grug::{Dec128_6, Int128, IsZero, MultiplyRatio, MutableCtx, Number as _, Response, Uint128},
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

    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);

    let mut oracle_querier = OracleQuerier::new_remote(oracle(ctx.querier), ctx.querier);

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

    let vault_equity = compute_user_equity(&mut oracle_querier, &perp_querier, &vault_user_state)?;

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

    ensure!(
        vault_user_state.margin >= amount_to_release,
        "insufficient vault margin to cover withdrawal: {} (margin) < {} (release)",
        vault_user_state.margin,
        amount_to_release
    );

    let end_time = ctx.block.timestamp + param.vault_cooldown_period;

    let unlock = Unlock {
        amount_to_release,
        end_time,
    };

    // Update vault state.
    vault_user_state
        .margin
        .checked_sub_assign(amount_to_release)?;
    state
        .vault_share_supply
        .checked_sub_assign(shares_to_burn)?;

    // Update user state.
    user_state.vault_shares.checked_sub_assign(shares_to_burn)?;
    user_state.unlocks.push_back(unlock);

    // Save all state changes to storage.
    STATE.save(ctx.storage, &state)?;
    USER_STATES.save(ctx.storage, ctx.sender, &user_state)?;
    USER_STATES.save(ctx.storage, ctx.contract, &vault_user_state)?;

    #[cfg(feature = "tracing")]
    {
        tracing::info!(
            user = %ctx.sender,
            %shares_to_burn,
            %amount_to_release,
            "Liquidity removal queued"
        );
    }

    Ok(Response::new().add_event(LiquidityUnlocking {
        user: ctx.sender,
        amount: amount_to_release,
        shares_burned: shares_to_burn,
        end_time,
    })?)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::state::{PARAM, STATE, USER_STATES},
        dango_types::{
            UsdValue,
            config::AppConfig,
            perps::{Param, State, Unlock, UserState},
        },
        grug::{Addr, Coins, Duration, MockContext, MockQuerier, ResultExt, Timestamp, Uint128},
        std::collections::VecDeque,
    };

    use super::remove_liquidity;

    const CONTRACT: Addr = Addr::mock(0);
    const USER: Addr = Addr::mock(1);

    fn default_param() -> Param {
        Param {
            vault_cooldown_period: Duration::from_seconds(86400), // 1 day
            max_unlocks: 10,
            ..Default::default()
        }
    }

    fn mock_querier() -> MockQuerier {
        MockQuerier::new()
            .with_app_config(AppConfig::default())
            .unwrap()
    }

    /// Pre-populate storage with PARAM, STATE, and USER_STATES.
    fn setup(
        storage: &mut dyn grug::Storage,
        param: &Param,
        vault_share_supply: u128,
        user_state: UserState,
        vault_margin: i128,
    ) {
        PARAM.save(storage, param).unwrap();

        STATE
            .save(storage, &State {
                vault_share_supply: Uint128::new(vault_share_supply),
                ..Default::default()
            })
            .unwrap();

        if !user_state.is_empty() {
            USER_STATES.save(storage, USER, &user_state).unwrap();
        }

        if vault_margin != 0 {
            USER_STATES
                .save(storage, CONTRACT, &UserState {
                    margin: UsdValue::new_int(vault_margin),
                    ..Default::default()
                })
                .unwrap();
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
        let param = default_param();
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(
            &mut ctx.storage,
            &param,
            1_000_000,
            UserState {
                vault_shares: Uint128::new(1_000_000),
                ..Default::default()
            },
            1,
        );

        remove_liquidity(ctx.as_mutable(), Uint128::new(1_000_000)).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let unlock = user_state.unlocks.back().unwrap();
        assert_eq!(unlock.amount_to_release, UsdValue::new_int(1));
    }

    // ---- Test 2: partial withdrawal ----
    #[test]
    fn partial_withdrawal() {
        let param = default_param();
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(
            &mut ctx.storage,
            &param,
            1_000_000,
            UserState {
                vault_shares: Uint128::new(500_000),
                ..Default::default()
            },
            1,
        );

        remove_liquidity(ctx.as_mutable(), Uint128::new(500_000)).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let unlock = user_state.unlocks.back().unwrap();
        assert!(unlock.amount_to_release > UsdValue::ZERO);
    }

    // ---- Test 3: zero shares rejected ----
    #[test]
    fn zero_shares_rejected() {
        let param = default_param();
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        setup(&mut ctx.storage, &param, 1_000_000, UserState::default(), 1);

        remove_liquidity(ctx.as_mutable(), Uint128::new(0))
            .should_fail_with_error("amount of shares to burn must be positive");
    }

    // ---- Test 7: max unlocks exceeded ----
    #[test]
    fn max_unlocks_exceeded() {
        let param = Param {
            max_unlocks: 2,
            vault_cooldown_period: Duration::from_seconds(86400),
            ..Default::default()
        };

        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        // Already at max_unlocks (2 existing unlocks).
        setup(
            &mut ctx.storage,
            &param,
            1_000_000,
            UserState {
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
            },
            1,
        );

        remove_liquidity(ctx.as_mutable(), Uint128::new(500_000))
            .should_fail_with_error("too many");
    }

    // ---- Test: small burn should not lose precision ----
    #[test]
    fn small_burn_precision() {
        let param = default_param();
        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());

        // Vault: $2 margin, 2M shares.
        // effective_supply = 2M + 1M = 3M
        // effective_equity = $2 + $1 = $3 (raw = 3_000_000)
        //
        // Burn 1 share.
        // Correct: floor(3_000_000 * 1 / 3_000_000) = 1 raw = $0.000001.
        // Buggy (divide-first): ratio = 1 * 10^6 / 3_000_000 = 0 → amount = $0.
        setup(
            &mut ctx.storage,
            &param,
            2_000_000,
            UserState {
                vault_shares: Uint128::new(1),
                ..Default::default()
            },
            2,
        );

        remove_liquidity(ctx.as_mutable(), Uint128::new(1)).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let unlock = user_state.unlocks.back().unwrap();
        assert_eq!(unlock.amount_to_release, UsdValue::new_raw(1));
    }

    // ---- Test 12: unlock end_time is correct ----
    #[test]
    fn unlock_end_time_correct() {
        let param = Param {
            vault_cooldown_period: Duration::from_seconds(172_800), // 2 days
            max_unlocks: 10,
            ..Default::default()
        };

        let mut ctx = MockContext::new()
            .with_querier(mock_querier())
            .with_contract(CONTRACT)
            .with_sender(USER)
            .with_funds(Coins::default());
        ctx.block.timestamp = Timestamp::from_seconds(1_000_000);

        setup(
            &mut ctx.storage,
            &param,
            1_000_000,
            UserState {
                vault_shares: Uint128::new(500_000),
                ..Default::default()
            },
            1,
        );

        remove_liquidity(ctx.as_mutable(), Uint128::new(500_000)).should_succeed();

        let user_state = USER_STATES.load(&ctx.storage, USER).unwrap();
        let unlock = user_state.unlocks.back().unwrap();
        assert_eq!(unlock.end_time, Timestamp::from_seconds(1_172_800));
    }
}
