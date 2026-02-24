use {
    super::{BANK, ORACLE, VIRTUAL_ASSETS, VIRTUAL_SHARES},
    crate::{NoCachePairQuerier, PAIR_STATES, STATE, core::compute_vault_equity},
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, bank,
        perps::{self, PairId, State, settlement_currency},
    },
    grug::{
        Coins, Message, MultiplyFraction, MutableCtx, Number as _, Order as IterationOrder,
        Response, Signed, StdResult, Timestamp, Uint128,
    },
};

pub fn deposit(ctx: MutableCtx, min_shares_to_mint: Option<Uint128>) -> anyhow::Result<Response> {
    // ---------------------------- 1. Preparation -----------------------------

    // Load state, create querier objects.
    let mut state = STATE.load(ctx.storage)?;

    let pair_querier = NoCachePairQuerier::new_local(ctx.storage);
    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // Find all the existing trading pairs.
    // TODO: optimize this. Ideally we don't do database iteration which is slow.
    let pair_ids = PAIR_STATES
        .keys(ctx.storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    // --------------------------- 2. Business logic ---------------------------

    let (deposit_amount, shares_to_mint) = _deposit(
        ctx.block.timestamp,
        ctx.funds,
        &state,
        &pair_ids,
        &pair_querier,
        &mut oracle_querier,
        min_shares_to_mint,
    )?;

    // Update global state.
    state.vault_margin.checked_add_assign(deposit_amount)?;
    (state.vault_share_supply).checked_add_assign(shares_to_mint)?;

    // ------------------------ 3. Apply state changes -------------------------

    // Save the updated global state.
    STATE.save(ctx.storage, &state)?;

    // Send a message to instruct the bank contract to mint the share token.
    // Note: if `shares_to_mint` is zero, the `Coins::one` constructor call errors,
    // as intended.
    Ok(Response::new().add_message(Message::execute(
        BANK,
        &bank::ExecuteMsg::Mint {
            to: ctx.sender,
            coins: Coins::one(perps::DENOM.clone(), shares_to_mint)?,
        },
        Coins::new(),
    )?))
}

/// The actual logic for handling the deposit.
/// Returns: 1) the amount of settlement currency that was deposited,
/// 2) the amount of share token to be minted, both in base unit.
fn _deposit(
    current_time: Timestamp,
    mut funds: Coins,
    state: &State,
    pair_ids: &[PairId],
    pair_querier: &NoCachePairQuerier,
    oracle_querier: &mut OracleQuerier,
    min_shares_to_mint: Option<Uint128>,
) -> anyhow::Result<(Uint128, Uint128)> {
    // Query the price of the settlement currency.
    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // ------------------------- Step 1. Check deposit -------------------------

    // Find how much settlement currency the user has deposited.
    let deposit_amount = funds.take(settlement_currency::DENOM.clone()).amount;

    // The user should not have deposited anything else.
    ensure!(funds.is_empty(), "unexpected deposit: {:?}", funds);

    // --------------------- Step 2. Compute vault equity ----------------------

    // Add virtual shares to the current vault share supply to arrive at the
    // effective supply.
    let effective_supply = state.vault_share_supply.checked_add(VIRTUAL_SHARES)?;

    // Compute the value of the vault's margin by multiplying its balance with
    // the settlement currency price.
    let vault_margin_value = Quantity::from_base(state.vault_margin, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // Compute the vault's equity. This equals the vault's margin plus its
    // unrealized PnL and funding.
    let vault_equity = compute_vault_equity(
        vault_margin_value,
        pair_ids,
        pair_querier,
        oracle_querier,
        current_time,
    )?;

    // Add virtual asset to vault equity to arrive at the effective equity.
    let effective_equity = vault_equity.checked_add(VIRTUAL_ASSETS)?;

    ensure!(
        effective_equity.is_positive(),
        "vault is in catastrophic loss! deposit disabled. effective equity: {effective_equity}"
    );

    // -------------------------- Step 3. Mint shares --------------------------

    // Compute the value of the settlement currency the user is depositing.
    let deposit_value = Quantity::from_base(deposit_amount, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // Compute the amount of shares to mint.
    let ratio = deposit_value.checked_div(effective_equity)?;
    let shares_to_mint =
        effective_supply.checked_mul_dec_floor(ratio.into_inner().checked_into_unsigned()?)?;

    if let Some(min_shares_to_mint) = min_shares_to_mint {
        ensure!(
            shares_to_mint >= min_shares_to_mint,
            "too few shares minted: {shares_to_mint} (actual) < {min_shares_to_mint} (expected)"
        );
    }

    Ok((deposit_amount, shares_to_mint))
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
        grug::{Coin, Udec128, Uint128, hash_map},
        std::collections::HashMap,
        test_case::test_case,
    };

    /// Helper: USDC oracle price at exactly $1 with precision 6.
    fn usdc_price_at_dollar() -> PrecisionedPrice {
        PrecisionedPrice::new(Udec128::new_percent(100), Timestamp::from_seconds(0), 6)
    }

    /// Helper: build `Coins` containing `amount` of the settlement currency (USDC).
    fn usdc_coins(amount: u128) -> Coins {
        Coins::one(settlement_currency::DENOM.clone(), amount).unwrap()
    }

    // ---- Test 1: first deposit into an empty vault (no pairs) ----
    // effective_supply = 0 + 1_000_000 = 1_000_000
    // vault_equity = $0, effective_equity = $0 + $1 = $1
    // deposit_value = $1, ratio = $1/$1 = 1.0
    // shares = floor(1_000_000 × 1.0) = 1_000_000
    #[test]
    fn first_deposit_empty_vault() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State::default();

        let (deposit_amount, shares) = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(1_000_000),
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(deposit_amount, Uint128::new(1_000_000));
        assert_eq!(shares, Uint128::new(1_000_000));
    }

    // ---- Test 2: second deposit of same size into a non-empty vault ----
    // effective_supply = 1_000_000 + 1_000_000 = 2_000_000
    // vault_equity = $1, effective_equity = $1 + $1 = $2
    // deposit_value = $1, ratio = $1/$2 = 0.5
    // shares = floor(2_000_000 × 0.5) = 1_000_000
    #[test]
    fn second_deposit_same_size() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(1_000_000),
            vault_share_supply: Uint128::new(1_000_000),
        };

        let (_, shares) = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(1_000_000),
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(1_000_000));
    }

    // ---- Test 3: zero deposit ----
    // _deposit itself handles zero gracefully (returns 0 shares).
    // The deposit() wrapper would reject via Coins::one(..., 0).
    #[test]
    fn zero_deposit() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State::default();

        let (deposit_amount, shares) = _deposit(
            Timestamp::from_seconds(0),
            Coins::new(),
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(deposit_amount, Uint128::new(0));
        assert_eq!(shares, Uint128::new(0));
    }

    // ---- Test 4: unexpected coins rejected ----
    // USDC + an extra denom should error.
    #[test]
    fn unexpected_coins_rejected() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State::default();

        let mut funds = usdc_coins(1_000_000);
        funds
            .insert(Coin {
                denom: eth::DENOM.clone(),
                amount: Uint128::new(100),
            })
            .unwrap();

        let err = _deposit(
            Timestamp::from_seconds(0),
            funds,
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap_err();

        assert!(err.to_string().contains("unexpected deposit"));
    }

    // ---- Test 5: catastrophic loss rejects deposit ----
    // margin=100 USDC, ETH PnL=-5000 → equity=-4900, effective_equity=-4899 → error
    #[test]
    fn catastrophic_loss_rejects_deposit() {
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

        // 100 USDC = 100_000_000 base units
        let state = State {
            vault_margin: Uint128::new(100_000_000),
            vault_share_supply: Uint128::new(100_000_000),
        };

        let err = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(1_000_000),
            &state,
            std::slice::from_ref(&eth::DENOM),
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap_err();

        assert!(err.to_string().contains("catastrophic loss"));
    }

    // ---- Test 6: min_shares passes ----
    // Empty vault, deposit 1 USDC → 1M shares. min=1M → passes.
    #[test]
    fn min_shares_passes() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State::default();

        let (_, shares) = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(1_000_000),
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            Some(Uint128::new(1_000_000)),
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(1_000_000));
    }

    // ---- Test 7: min_shares fails ----
    // Empty vault, deposit 1 USDC → 1M shares. min=1_000_001 → error.
    #[test]
    fn min_shares_fails() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State::default();

        let err = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(1_000_000),
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            Some(Uint128::new(1_000_001)),
        )
        .unwrap_err();

        assert!(err.to_string().contains("too few shares minted"));
    }

    // ---- Test 8: deposit with unrealized PnL ----
    // margin=10k USDC, supply=10B, ETH PnL=-5k → equity=$5000
    // Deposit 5k USDC → depositor gets "cheap" shares (equity < margin)
    //
    // effective_supply = 10_000_000_000 + 1_000_000 = 10_001_000_000
    // effective_equity = $5000 + $1 = $5001 (raw 5_001_000_000)
    // deposit_value = $5000 (raw 5_000_000_000)
    // ratio = 5_000_000_000_000_000 / 5_001_000_000 = 999_800 (truncated)
    // shares = floor(10_001_000_000 * 999_800 / 1_000_000) = 9_998_999_800
    #[test]
    fn deposit_with_unrealized_pnl() {
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

        let (_, shares) = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(5_000_000_000),
            &state,
            std::slice::from_ref(&eth::DENOM),
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        // Shares > deposit amount because vault equity is lower than margin
        // (depositor gets more shares per dollar when PnL is negative).
        assert_eq!(shares, Uint128::new(9_998_999_800));
    }

    // ---- Test 9: deposit with funding ----
    // margin=10k USDC, supply=10B, ETH: PnL=-5k, recorded funding=+20
    // vault_equity = 10000 + (-5000) + 20 = 5020
    // effective_equity = $5021 (raw 5_021_000_000)
    // deposit 5k USDC → deposit_value = $5000 (raw 5_000_000_000)
    // ratio = 5_000_000_000_000_000 / 5_021_000_000 = 995_817 (truncated)
    // shares = floor(10_001_000_000 * 995_817 / 1_000_000) = 9_959_165_817
    #[test]
    fn deposit_with_funding() {
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

        let (_, shares) = _deposit(
            Timestamp::from_seconds(100),
            usdc_coins(5_000_000_000),
            &state,
            std::slice::from_ref(&eth::DENOM),
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(9_959_165_817));
    }

    // ---- Test 10: multiple pairs ----
    // ETH: skew=10, oi_weighted_entry=20000, oracle=2500 → pnl=-5000
    // BTC: skew=-1, oi_weighted_entry=-50000, oracle=48000 → pnl=-2000
    // margin=10k, equity = 10000+(-5000)+(-2000) = 3000
    // effective_equity = $3001, deposit 1k USDC → deposit_value=$1000
    // effective_supply = 10_000_000_000 + 1_000_000 = 10_001_000_000
    // ratio = 1_000_000_000_000_000 / 3_001_000_000 = 333_222 (truncated)
    // shares = floor(10_001_000_000 * 333_222 / 1_000_000) = 3_332_553_222
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

        let (_, shares) = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(1_000_000_000),
            &state,
            &[eth::DENOM.clone(), btc::DENOM.clone()],
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(3_332_553_222));
    }

    // ---- Test 11: large deposit no overflow ----
    // 1B USDC vault + 1B USDC deposit. Both are 1_000_000_000 * 1_000_000 base.
    #[test]
    fn large_deposit_no_overflow() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let one_billion_usdc: u128 = 1_000_000_000 * 1_000_000;

        let state = State {
            vault_margin: Uint128::new(one_billion_usdc),
            vault_share_supply: Uint128::new(one_billion_usdc),
        };

        let (deposit_amount, shares) = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(one_billion_usdc),
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(deposit_amount, Uint128::new(one_billion_usdc));
        // With existing margin equal to deposit, shares should be close to supply
        // (slightly less due to virtual shares/assets dilution).
        assert!(shares > Uint128::new(0));
    }

    // ---- Test 12: non-dollar settlement price (parametric) ----
    // Empty vault, no pairs. Deposit 1 USDC.
    // At $0.99: deposit_value=$0.99, effective_equity=$1, ratio=0.99
    //   shares = floor(1_000_000 × 0.99) = 990_000
    // At $1.01: deposit_value=$1.01, effective_equity=$1, ratio=1.01
    //   shares = floor(1_000_000 × 1.01) = 1_010_000
    #[test_case(99, 990_000 ; "usdc below peg")]
    #[test_case(101, 1_010_000 ; "usdc above peg")]
    fn non_dollar_settlement_price(price_percent: u128, expected_shares: u128) {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(price_percent),
                Timestamp::from_seconds(0),
                6,
            ),
        });

        let state = State::default();

        let (_, shares) = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(1_000_000),
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(expected_shares));
    }

    // ---- Test 13: shares rounded floor ----
    // margin=2 USDC (2_000_000), supply=2_000_000, no pairs
    // effective_supply = 3_000_000, effective_equity = $3 (raw 3_000_000)
    // deposit 1 USDC: deposit_value = $1 (raw 1_000_000)
    // ratio = (1_000_000 * 1_000_000) / 3_000_000 = 333_333 (truncated from 333_333.33...)
    // shares = floor(3_000_000 * 333_333 / 1_000_000) = floor(999_999.0) = 999_999
    #[test]
    fn shares_rounded_floor() {
        let pair_querier = NoCachePairQuerier::new_mock(HashMap::new(), HashMap::new(), None);
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let state = State {
            vault_margin: Uint128::new(2_000_000),
            vault_share_supply: Uint128::new(2_000_000),
        };

        let (_, shares) = _deposit(
            Timestamp::from_seconds(0),
            usdc_coins(1_000_000),
            &state,
            &[],
            &pair_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        // Should be 999_999, not 1_000_000 (ceil) or 1_000_002 (if division ceiled).
        assert_eq!(shares, Uint128::new(999_999));
    }
}
