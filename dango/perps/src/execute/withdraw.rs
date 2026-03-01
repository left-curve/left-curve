use {
    crate::{
        NoCachePerpQuerier, PARAM, STATE, USER_STATES,
        core::compute_user_equity,
        execute::{BANK, ORACLE, VIRTUAL_ASSETS, VIRTUAL_SHARES},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, bank,
        perps::{self, Param, State, Unlock, UserState, settlement_currency},
    },
    grug::{
        Coins, IsZero, Message, MultiplyRatio, MutableCtx, Number as _, Response, Timestamp,
        Uint128,
    },
};

pub fn withdraw(ctx: MutableCtx) -> anyhow::Result<Response> {
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

    let (shares_to_burn, unlock) = _withdraw(
        ctx.block.timestamp,
        ctx.funds,
        &state,
        &param,
        &user_state,
        &vault_user_state,
        &perp_querier,
        &mut oracle_querier,
    )?;

    // Update global state.
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

/// The actual logic for handling the withdrawal.
///
/// Mutates: nothing (pure computation).
///
/// Returns: 1) the amount of shares to burn, 2) the unlock record.
fn _withdraw(
    current_time: Timestamp,
    mut funds: Coins,
    state: &State,
    param: &Param,
    user_state: &UserState,
    vault_user_state: &UserState,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
) -> anyhow::Result<(Uint128, Unlock)> {
    // Query the price of the settlement currency.
    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // -------------------- Step 1. Extract shares to burn ---------------------

    let shares_to_burn = funds.take(perps::DENOM.clone()).amount;

    ensure!(funds.is_empty(), "unexpected funds: {funds:?}");

    ensure!(shares_to_burn.is_non_zero(), "nothing to do");

    ensure!(
        user_state.unlocks.len() < param.max_unlocks,
        "too many pending unlocks"
    );

    // --------------------- Step 2. Compute vault equity ----------------------

    // Compute the vault's true equity including unrealized PnL and funding.
    let vault_margin_value = Quantity::from_base(state.vault_margin, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;
    let vault_equity = compute_user_equity(
        vault_margin_value,
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

    // Convert effective equity from USD to settlement currency human units,
    // then to base units.
    let vault_equity_in_settlement = effective_equity.checked_div(settlement_currency_price)?;
    let vault_equity_base =
        vault_equity_in_settlement.into_base_floor(settlement_currency::DECIMAL)?;

    // amount_to_release = floor(vault_equity_base * shares_to_burn / effective_supply)
    let amount_to_release =
        vault_equity_base.checked_multiply_ratio_floor(shares_to_burn, effective_supply)?;

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
        dango_types::{oracle::PrecisionedPrice, perps::settlement_currency},
        grug::{Coin, Duration, MockStorage, Udec128, Uint128, hash_map},
        std::collections::VecDeque,
        test_case::test_case,
    };

    /// Helper: USDC oracle price at exactly $1 with precision 6.
    fn usdc_price_at_dollar() -> PrecisionedPrice {
        PrecisionedPrice::new(Udec128::new_percent(100), Timestamp::from_seconds(0), 6)
    }

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
    // Deposit 1 USDC → 1M shares, withdraw all 1M shares → get back exactly 1 USDC.
    // state: margin=1_000_000, supply=1_000_000
    // effective_supply = 1M + 1M = 2M
    // vault_equity = $1, effective_equity = $2
    // vault_equity_base = $2 / $1 * 10^6 = 2_000_000
    // amount = floor(2_000_000 * 1_000_000 / 2_000_000) = 1_000_000
    #[test]
    fn first_withdrawal_symmetric() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let (shares, unlock) = _withdraw(
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
        assert_eq!(unlock.amount_to_release, Uint128::new(1_000_000));
    }

    // ---- Test 2: partial withdrawal ----
    // Same state, withdraw half the shares → half the equity.
    // amount = floor(2_000_000 * 500_000 / 2_000_000) = 500_000
    #[test]
    fn partial_withdrawal() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let (_, unlock) = _withdraw(
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

        assert_eq!(unlock.amount_to_release, Uint128::new(500_000));
    }

    // ---- Test 3: zero shares rejected ----
    #[test]
    fn zero_shares_rejected() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let err = _withdraw(
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
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
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

        let err = _withdraw(
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
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
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
                    amount_to_release: Uint128::new(100),
                    end_time: Timestamp::from_seconds(100),
                },
                Unlock {
                    amount_to_release: Uint128::new(200),
                    end_time: Timestamp::from_seconds(200),
                },
            ]),
            ..Default::default()
        };

        let err = _withdraw(
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

    // ---- Test 11: non-dollar settlement price ----
    // margin=10M, supply=10M, withdraw 1M shares, no pairs.
    // effective_supply = 11_000_000, effective_equity = vault_margin_value + $1
    //
    // At $0.99: vault_margin_value = $9.90, effective_equity = $10.90
    //   vault_equity_in_settlement = $10.90 / $0.99 = 11.010101 (truncated)
    //   vault_equity_base = 11_010_101
    //   amount = floor(11_010_101 * 1_000_000 / 11_000_000) = 1_000_918
    //
    // At $1.01: vault_margin_value = $10.10, effective_equity = $11.10
    //   vault_equity_in_settlement = $11.10 / $1.01 = 10.990099 (truncated)
    //   vault_equity_base = 10_990_099
    //   amount = floor(10_990_099 * 1_000_000 / 11_000_000) = 999_099
    #[test_case(99, 1_000_918 ; "usdc below peg")]
    #[test_case(101, 999_099 ; "usdc above peg")]
    fn non_dollar_settlement_price(price_percent: u128, expected_release: u128) {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(price_percent),
                Timestamp::from_seconds(0),
                6,
            ),
        });

        let state = State {
            vault_margin: Uint128::new(10_000_000),
            vault_share_supply: Uint128::new(10_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let (_, unlock) = _withdraw(
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

        assert_eq!(unlock.amount_to_release, Uint128::new(expected_release));
    }

    // ---- Test 12: unlock end_time is correct ----
    #[test]
    fn unlock_end_time_correct() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
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

        let (_, unlock) = _withdraw(
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

    // ---- Test 13: amount rounded floor ----
    // margin=10_000_001, supply=7_000_000, withdraw 3_000_000
    // effective_supply = 8_000_000, vault_equity = $10.000001, effective_equity = $11.000001
    // vault_equity_base = floor(11.000001 * 10^6) = 11_000_001
    // amount = floor(11_000_001 * 3_000_000 / 8_000_000) = floor(4_125_000.375) = 4_125_000
    #[test]
    fn amount_rounded_floor() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(10_000_001),
            vault_share_supply: Uint128::new(7_000_000),
            ..Default::default()
        };
        let param = default_param();
        let user_state = UserState::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let (_, unlock) = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(3_000_000),
            &state,
            &param,
            &user_state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(unlock.amount_to_release, Uint128::new(4_125_000));
    }
}
