use {
    super::{BANK, ORACLE, VIRTUAL_ASSETS, VIRTUAL_SHARES},
    crate::{
        NoCachePairQuerier, PAIR_STATES, PARAM, STATE, USER_STATES, core::compute_vault_equity,
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, bank,
        perps::{self, PairId, Param, State, Unlock, UserState, settlement_currency},
    },
    grug::{
        Coins, IsZero, Message, MultiplyRatio, MutableCtx, Number as _, Order as IterationOrder,
        Response, StdResult, Timestamp, Uint128,
    },
};

pub fn withdraw(ctx: MutableCtx) -> anyhow::Result<Response> {
    // ---------------------------- 1. Preparation -----------------------------

    let param = PARAM.load(ctx.storage)?;
    let mut state = STATE.load(ctx.storage)?;
    let mut user_state = USER_STATES
        .may_load(ctx.storage, ctx.sender)?
        .unwrap_or_default();

    let pair_querier = NoCachePairQuerier::new_local(ctx.storage);
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    let pair_ids = PAIR_STATES
        .keys(ctx.storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    // --------------------------- 2. Business logic ---------------------------

    let (shares_to_burn, unlock) = _withdraw(
        ctx.block.timestamp,
        ctx.funds,
        &state,
        &param,
        &user_state,
        &pair_ids,
        &pair_querier,
        &mut oracle_querier,
    )?;

    // Update global state.
    (state.vault_margin).checked_sub_assign(unlock.amount_to_release)?;
    (state.vault_share_supply).checked_sub_assign(shares_to_burn)?;

    // Update user state.
    user_state.unlocks.push(unlock);

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
/// Returns: 1) the amount of shares to burn, 2) the unlock record.
fn _withdraw(
    current_time: Timestamp,
    mut funds: Coins,
    state: &State,
    param: &Param,
    user_state: &UserState,
    pair_ids: &[PairId],
    pair_querier: &NoCachePairQuerier,
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

    let effective_supply = state.vault_share_supply.checked_add(VIRTUAL_SHARES)?;

    let vault_margin_value = Quantity::from_base(state.vault_margin, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    let vault_equity = compute_vault_equity(
        vault_margin_value,
        pair_ids,
        pair_querier,
        oracle_querier,
        current_time,
    )?;

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
        dango_types::{
            FundingPerUnit, Quantity, UsdValue,
            constants::{btc, eth},
            oracle::PrecisionedPrice,
            perps::{PairParam, PairState, settlement_currency},
        },
        grug::{Coin, Duration, Udec128, Uint128, hash_map},
        std::collections::HashMap,
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
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let (shares, unlock) = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(1_000_000),
            &state,
            &param,
            &user_state,
            &[],
            &pair_querier,
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
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let (_, unlock) = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(500_000),
            &state,
            &param,
            &user_state,
            &[],
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(unlock.amount_to_release, Uint128::new(500_000));
    }

    // ---- Test 3: zero shares rejected ----
    #[test]
    fn zero_shares_rejected() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let err = _withdraw(
            Timestamp::from_seconds(0),
            Coins::new(),
            &state,
            &param,
            &user_state,
            &[],
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("nothing to do"));
    }

    // ---- Test 4: unexpected coins rejected ----
    #[test]
    fn unexpected_coins_rejected() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

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
            &[],
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("unexpected"));
    }

    // ---- Test 5: catastrophic loss rejects withdrawal ----
    // margin=100 USDC, ETH PnL=-5000 → equity=-4900, effective_equity=-4899
    #[test]
    fn catastrophic_loss_rejects_withdrawal() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: Quantity::new_int(10),
                    oi_weighted_entry_price: UsdValue::new_int(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        let state = State {
            vault_margin: Uint128::new(100_000_000),
            vault_share_supply: Uint128::new(100_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let err = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(1_000_000),
            &state,
            &param,
            &user_state,
            std::slice::from_ref(&eth::DENOM),
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("catastrophic loss"));
    }

    // ---- Test 6: insufficient vault margin ----
    // margin=100 USDC (100_000_000), supply=100_000_000
    // ETH: skew=-10, oi_weighted_entry=-20000, oracle=2500
    //   vault_pnl = -20000 - 2500*(-10) = +5000
    // vault_equity = $100 + $5000 = $5100
    // effective_supply = 101_000_000, effective_equity = $5101
    // vault_equity_base = 5_101_000_000
    // Withdraw all 100M shares: amount = floor(5_101_000_000 * 100_000_000 / 101_000_000) = 5_050_495_049
    // vault_margin = 100_000_000 < 5_050_495_049 → error
    #[test]
    fn insufficient_vault_margin() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: Quantity::new_int(-10),
                    oi_weighted_entry_price: UsdValue::new_int(-20_000),
                    last_funding_time: Timestamp::from_seconds(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        let state = State {
            vault_margin: Uint128::new(100_000_000),
            vault_share_supply: Uint128::new(100_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let err = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(100_000_000),
            &state,
            &param,
            &user_state,
            std::slice::from_ref(&eth::DENOM),
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("insufficient"));
    }

    // ---- Test 7: max unlocks exceeded ----
    #[test]
    fn max_unlocks_exceeded() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
        };
        let param = Param {
            max_unlocks: 2,
            vault_cooldown_period: Duration::from_seconds(86400),
            ..Default::default()
        };

        // Already at max_unlocks (2 existing unlocks).
        let user_state = UserState {
            unlocks: vec![
                Unlock {
                    amount_to_release: Uint128::new(100),
                    end_time: Timestamp::from_seconds(100),
                },
                Unlock {
                    amount_to_release: Uint128::new(200),
                    end_time: Timestamp::from_seconds(200),
                },
            ],
            ..Default::default()
        };

        let err = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(500_000),
            &state,
            &param,
            &user_state,
            &[],
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap_err();

        assert!(err.to_string().contains("too many"));
    }

    // ---- Test 8: withdrawal with unrealized PnL ----
    // margin=10k USDC (10_000_000_000), supply=10_000_000_000
    // ETH: skew=10, oi_weighted_entry=20000, oracle=2500
    //   vault_pnl = 20000 - 2500*10 = -5000
    // vault_equity = $10000 + (-$5000) = $5000
    // effective_supply = 10_001_000_000, effective_equity = $5001
    // vault_equity_base = 5_001_000_000
    // Withdraw 5_000_000_000 shares:
    //   amount = floor(5_001_000_000 * 5_000_000_000 / 10_001_000_000) = 2_500_249_975
    #[test]
    fn withdrawal_with_unrealized_pnl() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: Quantity::new_int(10),
                    oi_weighted_entry_price: UsdValue::new_int(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        let state = State {
            vault_margin: Uint128::new(10_000_000_000),
            vault_share_supply: Uint128::new(10_000_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let (_, unlock) = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(5_000_000_000),
            &state,
            &param,
            &user_state,
            std::slice::from_ref(&eth::DENOM),
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(unlock.amount_to_release, Uint128::new(2_500_249_975));
    }

    // ---- Test 9: withdrawal with funding ----
    // margin=10k USDC (10_000_000_000), supply=10_000_000_000
    // ETH: skew=10, oi_weighted_entry=20000, oracle=2500
    //   vault_pnl = -5000
    //   funding: funding_per_unit=3, oi_weighted_entry_funding=10, no time elapsed
    //     recorded = 3*10 - 10 = 20
    // vault_equity = 10000 + (-5000) + 20 = $5020
    // effective_equity = $5021, effective_supply = 10_001_000_000
    // vault_equity_base = 5_021_000_000
    // Withdraw 5_000_000_000 shares:
    //   amount = floor(5_021_000_000 * 5_000_000_000 / 10_001_000_000) = 2_510_248_975
    #[test]
    fn withdrawal_with_funding() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: Quantity::new_int(10),
                    oi_weighted_entry_price: UsdValue::new_int(20_000),
                    funding_per_unit: FundingPerUnit::new_int(3),
                    oi_weighted_entry_funding: UsdValue::new_int(10),
                    last_funding_time: Timestamp::from_seconds(100),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
        });

        let state = State {
            vault_margin: Uint128::new(10_000_000_000),
            vault_share_supply: Uint128::new(10_000_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let (_, unlock) = _withdraw(
            Timestamp::from_seconds(100),
            share_coins(5_000_000_000),
            &state,
            &param,
            &user_state,
            std::slice::from_ref(&eth::DENOM),
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(unlock.amount_to_release, Uint128::new(2_510_248_975));
    }

    // ---- Test 10: multiple pairs ----
    // ETH: skew=10, oi_weighted_entry=20000, oracle=2500 → pnl=-5000
    // BTC: skew=-1, oi_weighted_entry=-50000, oracle=48000 → pnl=-2000
    // margin=10k, equity = 10000 + (-5000) + (-2000) = 3000
    // effective_equity = $3001, effective_supply = 10_001_000_000
    // vault_equity_base = 3_001_000_000
    // Withdraw 1_000_000_000 shares:
    //   amount = floor(3_001_000_000 * 1_000_000_000 / 10_001_000_000) = 300_069_993
    #[test]
    fn multiple_pairs() {
        let pair_querier = NoCachePairQuerier::new_mock(
            hash_map! {
                eth::DENOM.clone() => PairParam::default(),
                btc::DENOM.clone() => PairParam::default(),
            },
            hash_map! {
                eth::DENOM.clone() => PairState {
                    skew: Quantity::new_int(10),
                    oi_weighted_entry_price: UsdValue::new_int(20_000),
                    last_funding_time: Timestamp::from_seconds(0),
                    ..Default::default()
                },
                btc::DENOM.clone() => PairState {
                    skew: Quantity::new_int(-1),
                    oi_weighted_entry_price: UsdValue::new_int(-50_000),
                    last_funding_time: Timestamp::from_seconds(0),
                    ..Default::default()
                },
            },
            None,
        );
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
            eth::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(250_000),
                Timestamp::from_seconds(0),
                18,
            ),
            btc::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(4_800_000),
                Timestamp::from_seconds(0),
                8,
            ),
        });

        let state = State {
            vault_margin: Uint128::new(10_000_000_000),
            vault_share_supply: Uint128::new(10_000_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let (_, unlock) = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(1_000_000_000),
            &state,
            &param,
            &user_state,
            &[eth::DENOM.clone(), btc::DENOM.clone()],
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(unlock.amount_to_release, Uint128::new(300_069_993));
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
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
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
        };
        let param = default_param();
        let user_state = UserState::default();

        let (_, unlock) = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(1_000_000),
            &state,
            &param,
            &user_state,
            &[],
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(unlock.amount_to_release, Uint128::new(expected_release));
    }

    // ---- Test 12: unlock end_time is correct ----
    #[test]
    fn unlock_end_time_correct() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
        };
        let param = Param {
            vault_cooldown_period: Duration::from_seconds(172_800), // 2 days
            max_unlocks: 10,
            ..Default::default()
        };
        let user_state = UserState::default();

        let (_, unlock) = _withdraw(
            Timestamp::from_seconds(1_000_000),
            share_coins(500_000),
            &state,
            &param,
            &user_state,
            &[],
            &pair_querier,
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
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(10_000_001),
            vault_share_supply: Uint128::new(7_000_000),
        };
        let param = default_param();
        let user_state = UserState::default();

        let (_, unlock) = _withdraw(
            Timestamp::from_seconds(0),
            share_coins(3_000_000),
            &state,
            &param,
            &user_state,
            &[],
            &pair_querier,
            &mut oracle_querier,
        )
        .unwrap();

        assert_eq!(unlock.amount_to_release, Uint128::new(4_125_000));
    }
}
