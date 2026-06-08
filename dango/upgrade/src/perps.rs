//! Perps storage migrations applied during chain upgrades.

#![allow(dead_code)]

use {
    dango_perps::{
        state::USER_STATES,
        trade::{ResizeReduceOnlyOutcome, compute_resize_reduce_only_outcome},
    },
    dango_types::constants::{perp_btc, perp_eth, perp_hype, perp_sol, perp_xag, perp_xau},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    grug_types::{Addr, Order, StdResult, Storage, addr},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Pre-migration perps storage shapes.
mod legacy_perps {
    // Add content here.
}

pub fn do_perps_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let perps_address = {
        let chain_id = CHAIN_ID.load(&storage)?;
        match chain_id.as_str() {
            MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
            TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
            _ => panic!("unknown chain id: {chain_id}"),
        }
    };

    let mut perps_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    sweep_reduce_only_orders(&mut perps_storage)?;

    Ok(())
}

/// Re-clamp every user's resting reduce-only orders to their current position,
/// across all 6 perp pairs — a one-time cleanup of the reduce-only "ghost"
/// orders that the match-time clamp left resting on the book (a flat user's
/// inert reduce-only orders are never swept by the per-transaction re-size,
/// since they produce no fill and so never enter the affected-user set).
///
/// Reuses the same scanner the contract runs on every position change
/// (`compute_resize_reduce_only_outcome`): a flat user has every reduce-only
/// order cancelled, an over-extended one has them shrunk to the position, and a
/// healthy one is left untouched. Events are suppressed — the upgrade handler
/// has no event channel, and order-book events are not indexed; consumers read
/// chain state, which this corrects (orders removed, liquidity depth and
/// reserved margin released).
fn sweep_reduce_only_orders(storage: &mut dyn Storage) -> StdResult<()> {
    let pairs = [
        &*perp_btc::DENOM,
        &*perp_eth::DENOM,
        &*perp_sol::DENOM,
        &*perp_hype::DENOM,
        &*perp_xau::DENOM,
        &*perp_xag::DENOM,
    ];

    // Collect the user set first: the loop below mutates `USER_STATES`, so we
    // must not hold an iterator over it while doing so (mirrors
    // `compute_cancel_all_orders_outcome`).
    let users = USER_STATES
        .range(storage, None, None, Order::Ascending)
        .map(|res| res.map(|(addr, _)| addr))
        .collect::<StdResult<Vec<Addr>>>()?;

    for user in users {
        let original = USER_STATES.load(storage, user)?;
        let mut state = original.clone();

        // The scanner reads the position for `pair` from `state` and the orders
        // from storage, mutating the book in place. Reserved margin and the open
        // order count are account-wide, so thread the running `state` across all
        // pairs and persist once at the end.
        for pair in pairs {
            let ResizeReduceOnlyOutcome { user_state, .. } =
                compute_resize_reduce_only_outcome(storage, user, pair, &state, None)?;
            state = user_state;
        }

        // Only write back users we actually changed — most hold no reduce-only
        // orders, so the scanner left them untouched.
        if state != original {
            if state.is_empty() {
                USER_STATES.remove(storage, user)?;
            } else {
                USER_STATES.save(storage, user, &state)?;
            }
        }
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::{ASKS, FundingPerUnit, LimitOrder, Quantity, UsdPrice, UsdValue},
        dango_perps::state::PAIR_PARAMS,
        dango_types::perps::{PairParam, Position, UserState},
        grug_math::Uint64,
        grug_types::{MockContext, Timestamp},
        std::collections::BTreeMap,
    };

    // Three accounts: a flat one carrying stale reduce-only orders, a correctly
    // sized one, and an over-extended one.
    const FLAT: Addr = Addr::mock(1);
    const HEALTHY: Addr = Addr::mock(2);
    const OVERSIZED: Addr = Addr::mock(3);

    /// Save a reduce-only `perp/btcusd` ask.
    fn save_ro_ask(
        storage: &mut dyn Storage,
        user: Addr,
        order_id: u64,
        price: i128,
        size: i128,
        reserved: i128,
    ) {
        let key = (
            perp_btc::DENOM.clone(),
            UsdPrice::new_int(price),
            Uint64::new(order_id),
        );
        let order = LimitOrder {
            user,
            size: Quantity::new_int(size),
            reduce_only: true,
            reserved_margin: UsdValue::new_int(reserved),
            created_at: Timestamp::from_nanos(0),
            tp: None,
            sl: None,
            client_order_id: None,
        };
        ASKS.save(storage, key, &order).unwrap();
    }

    /// A `perp/btcusd` long of the given (integer) size.
    fn long(size: i128) -> BTreeMap<dango_order_book::PairId, Position> {
        BTreeMap::from([(perp_btc::DENOM.clone(), Position {
            size: Quantity::new_int(size),
            entry_price: UsdPrice::new_int(2_000),
            entry_funding_per_unit: FundingPerUnit::ZERO,
            conditional_order_above: None,
            conditional_order_below: None,
        })])
    }

    fn seed_user(
        storage: &mut dyn Storage,
        user: Addr,
        margin: i128,
        reserved: i128,
        open_order_count: usize,
        positions: BTreeMap<dango_order_book::PairId, Position>,
    ) {
        let state = UserState {
            margin: UsdValue::new_int(margin),
            positions,
            reserved_margin: UsdValue::new_int(reserved),
            open_order_count,
            ..Default::default()
        };
        USER_STATES.save(storage, user, &state).unwrap();
    }

    /// The user's resting asks, in book order.
    fn asks_of(storage: &dyn Storage, user: Addr) -> Vec<LimitOrder> {
        ASKS.idx
            .user
            .prefix(user)
            .range(storage, None, None, Order::Ascending)
            .map(|res| res.unwrap().1)
            .collect()
    }

    /// A flat account carrying many identical reduce-only asks — their sizes
    /// summing far above its (zero) position — has every order cancelled and its
    /// reserved margin released. A correctly-sized account is left untouched, and
    /// an over-extended one is shrunk to its position.
    #[test]
    fn sweeps_reduce_only_ghosts() {
        let mut ctx = MockContext::new();
        let storage = &mut ctx.storage;

        PAIR_PARAMS
            .save(storage, &*perp_btc::DENOM, &PairParam::default())
            .unwrap();

        // FLAT: no position, but 50 reduce-only sells of -1 still resting, each
        // reserving 6 — total 300 reserved against only 10 margin.
        for i in 0..50u64 {
            save_ro_ask(storage, FLAT, i + 1, 60_000, -1, 6);
        }
        seed_user(storage, FLAT, 10, 300, 50, BTreeMap::new());

        // HEALTHY: long 5 with a correctly-sized reduce-only sell of -5.
        save_ro_ask(storage, HEALTHY, 1_000, 60_000, -5, 30);
        seed_user(storage, HEALTHY, 100, 30, 1, long(5));

        // OVERSIZED: long 3 with an over-extended reduce-only sell of -10.
        save_ro_ask(storage, OVERSIZED, 2_000, 60_000, -10, 60);
        seed_user(storage, OVERSIZED, 100, 60, 1, long(3));

        sweep_reduce_only_orders(storage).unwrap();

        // FLAT: every order swept, reserved margin released, margin untouched.
        // The account survives (margin is non-zero), just with a clean book.
        assert!(
            asks_of(storage, FLAT).is_empty(),
            "all stale orders cancelled"
        );
        let flat = USER_STATES.load(storage, FLAT).unwrap();
        assert_eq!(flat.open_order_count, 0);
        assert_eq!(flat.reserved_margin, UsdValue::ZERO);
        assert_eq!(flat.margin, UsdValue::new_int(10), "margin untouched");
        assert!(flat.positions.is_empty());

        // HEALTHY: the correctly-sized order is left exactly as it was.
        let healthy = asks_of(storage, HEALTHY);
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].size, Quantity::new_int(-5));
        assert_eq!(
            USER_STATES.load(storage, HEALTHY).unwrap().open_order_count,
            1
        );

        // OVERSIZED: the order is shrunk from -10 to the position (-3).
        let oversized = asks_of(storage, OVERSIZED);
        assert_eq!(oversized.len(), 1);
        assert_eq!(
            oversized[0].size,
            Quantity::new_int(-3),
            "shrunk to position"
        );
        assert!(
            oversized[0].reserved_margin < UsdValue::new_int(60),
            "reserved margin released proportionally"
        );
    }
}
