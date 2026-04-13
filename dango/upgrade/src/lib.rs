use {
    dango_perps::state::{ASKS, BIDS, DEPTHS, LONGS, PAIR_STATES, SHORTS, STATE, USER_STATES},
    dango_types::{FundingRate, Quantity, UsdValue},
    grug::{Addr, BlockInfo, Order as IterationOrder, StdResult, Storage, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&*storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    let mut perps_storage =
        StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    let _total_liability = clear_perps_state(&mut perps_storage)?;

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
    }

    tracing::info!("All invariants passed");

    Ok(())
}
