//! Post-exploit chain upgrade handler (2026-04-13 insurance fund drain).
//!
//! The chain was halted after detection. During downtime, asset prices moved
//! but users couldn't close positions, so the only fair restart is to
//! close everything and let users re-enter at current prices.
//!
//! The upgrade runs in three phases:
//!
//! 1. **Claw back attacker funds.** Both attacker accounts still hold ~$1M
//!    USDC on-chain (the rest was bridged to Ethereum). Zero their bank
//!    balances and credit the sum to the perps contract.
//!
//! 2. **Clear all perps state.** Cancel every resting order (BIDS, ASKS,
//!    DEPTHS), close every position (LONGS, SHORTS, USER_STATES.positions),
//!    zero OI and funding rates on all pairs, and zero the insurance fund.
//!    User margins, vault shares, and pending unlocks are preserved — those
//!    are what users are owed.
//!
//! 3. **Log the shortfall.** After claw-back, the perps contract's USDC
//!    balance is still less than total user liabilities (margin + unlocks)
//!    by roughly the amount the attacker bridged off-chain. The shortfall
//!    is logged via `tracing::warn!` so the team can bridge the returned
//!    funds back and donate them through `MaintainerMsg::Donate`.

use {
    dango_bank::BALANCES,
    dango_perps::state::{ASKS, BIDS, DEPTHS, LONGS, PAIR_STATES, SHORTS, STATE, USER_STATES},
    dango_types::{
        FundingRate, Quantity, UsdValue,
        constants::usdc,
        perps::{SETTLEMENT_CURRENCY_PRICE, settlement_currency},
    },
    grug::{
        Addr, BlockInfo, Denom, Number, NumberConst, Order as IterationOrder, StdResult, Storage,
        Uint128, addr,
    },
    grug_app::{AppResult, CHAIN_ID, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

const ATTACKER_1: Addr = addr!("023ef9e3e20caca6ef3743cbfba6469d69978999");
const ATTACKER_2: Addr = addr!("0e85f43a9e45a7c8835ded188890b7e57033b78f");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&*storage)?;
    let config = CONFIG.load(&*storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    // Phase 1: Claw back attacker funds (bank storage scope).
    let usdc_denom = usdc::DENOM.clone();
    {
        let mut bank_storage =
            StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &config.bank]);
        claw_back_attacker_funds(&mut bank_storage, &perps_address, &usdc_denom)?;
    }

    // Phase 2: Clear perps state (perps storage scope).
    let total_liability = {
        let mut perps_storage =
            StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &perps_address]);
        clear_perps_state(&mut perps_storage)?
    };

    // Phase 3: Calculate and log shortfall.
    {
        let bank_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &config.bank]);
        log_shortfall(&bank_storage, &perps_address, &usdc_denom, total_liability)?;
    }

    Ok(())
}

/// Zero out USDC balances of the two attacker accounts and add the sum to
/// the perps contract's balance. Operates on the bank contract's storage.
fn claw_back_attacker_funds(
    storage: &mut dyn Storage,
    perps_address: &Addr,
    usdc_denom: &Denom,
) -> StdResult<Uint128> {
    let bal1 = BALANCES
        .may_load(storage, (&ATTACKER_1, usdc_denom))?
        .unwrap_or_default();
    let bal2 = BALANCES
        .may_load(storage, (&ATTACKER_2, usdc_denom))?
        .unwrap_or_default();

    let clawed_back = bal1.checked_add(bal2)?;

    // Zero attacker balances.
    BALANCES.save(storage, (&ATTACKER_1, usdc_denom), &Uint128::ZERO)?;
    BALANCES.save(storage, (&ATTACKER_2, usdc_denom), &Uint128::ZERO)?;

    // Credit the clawed-back amount to the perps contract.
    let perps_bal = BALANCES
        .may_load(storage, (perps_address, usdc_denom))?
        .unwrap_or_default();
    BALANCES.save(
        storage,
        (perps_address, usdc_denom),
        &perps_bal.checked_add(clawed_back)?,
    )?;

    tracing::info!(
        attacker_1 = %bal1,
        attacker_2 = %bal2,
        total_clawed_back = %clawed_back,
        new_perps_balance = %(perps_bal.checked_add(clawed_back)?),
        "Clawed back attacker USDC funds to perps contract"
    );

    Ok(clawed_back)
}

/// Read the perps contract's USDC balance, convert to UsdValue, and log the
/// shortfall (liability minus asset). This is the amount that must be bridged
/// back and donated to the perps contract to make users whole.
fn log_shortfall(
    storage: &dyn Storage,
    perps_address: &Addr,
    usdc_denom: &Denom,
    total_liability: UsdValue,
) -> StdResult<()> {
    let perps_usdc_balance = BALANCES
        .may_load(storage, (perps_address, usdc_denom))?
        .unwrap_or_default();

    // Convert raw USDC (base units) to UsdValue via the same path as deposit.rs:
    // Quantity::from_base(amount, decimals) * SETTLEMENT_CURRENCY_PRICE
    let asset = Quantity::from_base(perps_usdc_balance, settlement_currency::DECIMAL)?
        .checked_mul(SETTLEMENT_CURRENCY_PRICE)?;

    let shortfall = total_liability.checked_sub(asset)?;

    if shortfall.is_positive() {
        tracing::warn!(
            %total_liability,
            perps_usdc_balance = %perps_usdc_balance,
            asset_usd = %asset,
            %shortfall,
            "!!! SHORTFALL: perps contract liabilities exceed assets. \
             Bridge the returned funds back to dango and donate to the perps contract. !!!"
        );
    } else {
        tracing::info!(
            %total_liability,
            perps_usdc_balance = %perps_usdc_balance,
            asset_usd = %asset,
            %shortfall,
            "No shortfall"
        );
    }

    Ok(())
}

/// Close all positions, cancel all orders, zero out OI/insurance fund.
///
/// Returns `total_liability`: sum of all user margins + all pending unlock
/// amounts. This is the amount the perps contract owes its users.
fn clear_perps_state(storage: &mut dyn Storage) -> StdResult<UsdValue> {
    // -------------------------------------------------------------------------
    // 1. Clear order book and position indexes
    // -------------------------------------------------------------------------

    BIDS.clear_all(storage);
    ASKS.clear_all(storage);
    LONGS.clear(storage, None, None);
    SHORTS.clear(storage, None, None);
    DEPTHS.clear(storage, None, None);

    tracing::info!("Cleared BIDS, ASKS, LONGS, SHORTS, DEPTHS");

    // -------------------------------------------------------------------------
    // 2. Reset user states: clear positions, zero reserved_margin/open_order_count
    // -------------------------------------------------------------------------

    let user_states: Vec<_> = USER_STATES
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let user_count = user_states.len();
    let mut total_margin = UsdValue::ZERO;
    let mut total_unlocks = UsdValue::ZERO;

    for (addr, mut user_state) in user_states {
        user_state.positions.clear();
        user_state.reserved_margin = UsdValue::ZERO;
        user_state.open_order_count = 0;

        // If the account is the attacker's, additionally zero out the margin.
        if [ATTACKER_1, ATTACKER_2].contains(&addr) {
            user_state.margin = UsdValue::ZERO;
        }

        // Accumulate liabilities (margin owed + pending vault unlock releases).
        total_margin.checked_add_assign(user_state.margin)?;
        for unlock in &user_state.unlocks {
            total_unlocks.checked_add_assign(unlock.amount_to_release)?;
        }

        USER_STATES.save(storage, addr, &user_state)?;
    }

    let total_liability = total_margin.checked_add(total_unlocks)?;

    tracing::info!(
        user_count,
        %total_margin,
        %total_unlocks,
        %total_liability,
        "Reset user states"
    );

    // -------------------------------------------------------------------------
    // 3. Zero out pair state OI and funding rate
    // -------------------------------------------------------------------------

    let pair_states: Vec<_> = PAIR_STATES
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<_>>()?;

    let pair_count = pair_states.len();

    for (pair_id, mut pair_state) in pair_states {
        pair_state.long_oi = Quantity::ZERO;
        pair_state.short_oi = Quantity::ZERO;
        pair_state.funding_rate = FundingRate::ZERO;
        PAIR_STATES.save(storage, &pair_id, &pair_state)?;
    }

    tracing::info!(pair_count, "Zeroed pair state OI and funding rates");

    // -------------------------------------------------------------------------
    // 4. Zero insurance fund
    // -------------------------------------------------------------------------

    let mut state = STATE.load(storage)?;
    state.insurance_fund = UsdValue::ZERO;
    STATE.save(storage, &state)?;

    tracing::info!("Zeroed insurance fund");

    // -------------------------------------------------------------------------
    // 5. Assert invariants
    // -------------------------------------------------------------------------

    assert_invariants(storage)?;

    Ok(total_liability)
}

fn assert_invariants(storage: &dyn Storage) -> StdResult<()> {
    // All pair states must have zero OI.
    for entry in PAIR_STATES.range(storage, None, None, IterationOrder::Ascending) {
        let (pair_id, ps) = entry?;
        assert!(
            ps.long_oi.is_zero() && ps.short_oi.is_zero(),
            "pair {pair_id} still has nonzero OI: long={}, short={}",
            ps.long_oi,
            ps.short_oi,
        );
    }

    // Insurance fund must be zero.
    let state = STATE.load(storage)?;
    assert!(state.insurance_fund.is_zero(), "insurance fund not zero");

    // No user should have positions, reserved_margin, or open orders.
    // The attacker must additionally have the margin zeroed out.
    for entry in USER_STATES.range(storage, None, None, IterationOrder::Ascending) {
        let (addr, us) = entry?;

        assert!(
            us.positions.is_empty(),
            "user {addr} still has {} positions",
            us.positions.len(),
        );
        assert!(
            us.reserved_margin.is_zero(),
            "user {addr} still has reserved margin: {}",
            us.reserved_margin,
        );
        assert!(
            us.open_order_count == 0,
            "user {addr} still has {} open orders",
            us.open_order_count,
        );

        if [ATTACKER_1, ATTACKER_2].contains(&addr) {
            assert!(
                us.margin.is_zero(),
                "attacker {addr} still has a non-zero margin of {}",
                us.margin
            );
        }
    }

    tracing::info!("All invariants passed");

    Ok(())
}
