use {
    crate::{
        NoCachePerpQuerier, STATE, USER_STATES,
        core::compute_user_equity,
        execute::{BANK, ORACLE, VIRTUAL_ASSETS, VIRTUAL_SHARES},
    },
    anyhow::ensure,
    dango_oracle::OracleQuerier,
    dango_types::{
        Quantity, bank,
        perps::{self, State, UserState, settlement_currency},
    },
    grug::{Coins, Message, MultiplyFraction, MutableCtx, Number as _, Response, Signed, Uint128},
};

/// Add liquidity to the counterparty vault.
///
/// Mutates: `STATE` (vault_margin, vault_share_supply).
///
/// Returns: `Response` with a bank mint message.
pub fn add_liquidity(
    ctx: MutableCtx,
    min_shares_to_mint: Option<Uint128>,
) -> anyhow::Result<Response> {
    // ---------------------------- 1. Preparation -----------------------------

    let mut state = STATE.load(ctx.storage)?;

    ensure!(
        !state.vault_margin.is_negative(),
        "deposits paused: unresolved ADL deficit"
    );

    let vault_user_state = USER_STATES
        .may_load(ctx.storage, ctx.contract)?
        .unwrap_or_default();

    let perp_querier = NoCachePerpQuerier::new_local(ctx.storage);

    let mut oracle_querier = OracleQuerier::new_remote(ORACLE, ctx.querier);

    // --------------------------- 2. Business logic ---------------------------

    let shares_to_mint = _add_liquidity(
        ctx.funds,
        &mut state,
        &vault_user_state,
        &perp_querier,
        &mut oracle_querier,
        min_shares_to_mint,
    )?;

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

/// The actual logic for handling the add-liquidity operation.
///
/// Mutates: `state` (vault_margin, vault_share_supply).
///
/// Returns: the amount of share token to be minted in base units.
fn _add_liquidity(
    mut funds: Coins,
    state: &mut State,
    vault_user_state: &UserState,
    perp_querier: &NoCachePerpQuerier,
    oracle_querier: &mut OracleQuerier,
    min_shares_to_mint: Option<Uint128>,
) -> anyhow::Result<Uint128> {
    // Query the price of the settlement currency.
    let settlement_currency_price =
        oracle_querier.query_price_for_perps(&settlement_currency::DENOM)?;

    // ------------------------- Step 1. Check deposit -------------------------

    // Find how much settlement currency the user has deposited.
    let deposit_amount = funds.take(settlement_currency::DENOM.clone()).amount;

    // The user should not have deposited anything else.
    ensure!(funds.is_empty(), "unexpected deposit: {:?}", funds);

    // --------------------- Step 2. Compute vault equity ----------------------

    // vault_margin is already UsdValue — no base→USD conversion needed.
    let vault_equity = compute_user_equity(
        state.vault_margin,
        vault_user_state,
        perp_querier,
        oracle_querier,
    )?;

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

    // Compute the USD value of the settlement currency the user is depositing.
    let deposit_value = Quantity::from_base(deposit_amount, settlement_currency::DECIMAL)?
        .checked_mul(settlement_currency_price)?;

    // Compute the amount of shares to mint.
    let ratio = deposit_value
        .checked_div(effective_equity)?
        .into_inner()
        .checked_into_unsigned()?;
    let shares_to_mint = effective_supply.checked_mul_dec_floor(ratio)?;

    if let Some(min_shares_to_mint) = min_shares_to_mint {
        ensure!(
            shares_to_mint >= min_shares_to_mint,
            "too few shares minted: {shares_to_mint} (actual) < {min_shares_to_mint} (expected)"
        );
    }

    // Update global state.
    state.vault_margin.checked_add_assign(deposit_value)?;
    (state.vault_share_supply).checked_add_assign(shares_to_mint)?;

    Ok(shares_to_mint)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::{
            UsdValue,
            oracle::PrecisionedPrice,
            perps::{UserState, settlement_currency},
        },
        grug::{Coin, MockStorage, Timestamp, Udec128, Uint128, hash_map},
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
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let mut state = State::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let shares = _add_liquidity(
            usdc_coins(1_000_000),
            &mut state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(1_000_000));
    }

    // ---- Test 2: second deposit of same size into a non-empty vault ----
    // effective_supply = 1_000_000 + 1_000_000 = 2_000_000
    // vault_equity = $1, effective_equity = $1 + $1 = $2
    // deposit_value = $1, ratio = $1/$2 = 0.5
    // shares = floor(2_000_000 × 0.5) = 1_000_000
    #[test]
    fn second_deposit_same_size() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let mut state = State {
            vault_margin: UsdValue::new_int(1),
            vault_share_supply: Uint128::new(1_000_000),
            ..Default::default()
        };
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let shares = _add_liquidity(
            usdc_coins(1_000_000),
            &mut state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(1_000_000));
    }

    // ---- Test 3: zero deposit ----
    // _add_liquidity itself handles zero gracefully (returns 0 shares).
    // The add_liquidity() wrapper would reject via Coins::one(..., 0).
    #[test]
    fn zero_add_liquidity() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let mut state = State::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let shares = _add_liquidity(
            Coins::new(),
            &mut state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(0));
    }

    // ---- Test 4: unexpected coins rejected ----
    // USDC + an extra denom should error.
    #[test]
    fn unexpected_coins_rejected() {
        use dango_types::constants::eth;

        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let mut state = State::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let mut funds = usdc_coins(1_000_000);
        funds
            .insert(Coin {
                denom: eth::DENOM.clone(),
                amount: Uint128::new(100),
            })
            .unwrap();

        let err = _add_liquidity(
            funds,
            &mut state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap_err();

        assert!(err.to_string().contains("unexpected deposit"));
    }

    // TODO(order-book): Tests 5, 8, 9, 10 depend on vault equity accounting
    // with unrealized PnL and funding across trading pairs. These will be
    // re-enabled once the new `compute_vault_equity` (treating the vault as
    // a regular trader) is implemented.

    // ---- Test 6: min_shares passes ----
    // Empty vault, deposit 1 USDC → 1M shares. min=1M → passes.
    #[test]
    fn min_shares_passes() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let mut state = State::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let shares = _add_liquidity(
            usdc_coins(1_000_000),
            &mut state,
            &vault_user_state,
            &perp_querier,
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
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let mut state = State::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let err = _add_liquidity(
            usdc_coins(1_000_000),
            &mut state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
            Some(Uint128::new(1_000_001)),
        )
        .unwrap_err();

        assert!(err.to_string().contains("too few shares minted"));
    }

    // ---- Test 11: large deposit no overflow ----
    // 1B USDC vault + 1B USDC deposit. Both are 1_000_000_000 * 1_000_000 base.
    #[test]
    fn large_deposit_no_overflow() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let one_billion_usdc: u128 = 1_000_000_000 * 1_000_000;

        let mut state = State {
            vault_margin: UsdValue::new_int(1_000_000_000),
            vault_share_supply: Uint128::new(one_billion_usdc),
            ..Default::default()
        };
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let shares = _add_liquidity(
            usdc_coins(one_billion_usdc),
            &mut state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

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
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => PrecisionedPrice::new(
                Udec128::new_percent(price_percent),
                Timestamp::from_seconds(0),
                6,
            ),
        });

        let mut state = State::default();
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let shares = _add_liquidity(
            usdc_coins(1_000_000),
            &mut state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        assert_eq!(shares, Uint128::new(expected_shares));
    }

    // ---- Test 13: shares rounded floor ----
    // margin=$2, supply=2_000_000, no pairs
    // effective_supply = 3_000_000, effective_equity = $3
    // deposit 1 USDC: deposit_value = $1
    // ratio = $1 / $3 = 0.333333...
    // shares = floor(3_000_000 × 0.333333) = 999_999
    #[test]
    fn shares_rounded_floor() {
        let storage = MockStorage::new();
        let mut oracle_querier = OracleQuerier::new_mock(hash_map! {
            settlement_currency::DENOM.clone() => usdc_price_at_dollar(),
        });

        let mut state = State {
            vault_margin: UsdValue::new_int(2),
            vault_share_supply: Uint128::new(2_000_000),
            ..Default::default()
        };
        let vault_user_state = UserState::default();
        let perp_querier = NoCachePerpQuerier::new_local(&storage);

        let shares = _add_liquidity(
            usdc_coins(1_000_000),
            &mut state,
            &vault_user_state,
            &perp_querier,
            &mut oracle_querier,
            None,
        )
        .unwrap();

        // Should be 999_999, not 1_000_000 (ceil) or 1_000_002 (if division ceiled).
        assert_eq!(shares, Uint128::new(999_999));
    }
}
