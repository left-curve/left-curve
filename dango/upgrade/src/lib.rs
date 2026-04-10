use {
    dango_perps::state::OrderKey,
    dango_types::{
        Quantity, UsdValue,
        perps::{self, ChildOrder},
    },
    grug::{Addr, BlockInfo, Map, Order as IterationOrder, StdResult, Storage, Timestamp, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("d04b99adca5d3d31a1e7bc72fd606202f1e2fc69");

/// Legacy types matching the pre-upgrade Borsh layout.
mod legacy {
    use super::*;

    /// The `LimitOrder` struct before this upgrade, which does not contain the
    /// `client_order_id` field.
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct LimitOrder {
        pub user: Addr,
        pub size: Quantity,
        pub reduce_only: bool,
        pub reserved_margin: UsdValue,
        pub created_at: Timestamp,
        pub tp: Option<ChildOrder>,
        pub sl: Option<ChildOrder>,
    }

    /// Plain Map with the same namespace as the `BIDS` IndexedMap primary key.
    pub const BIDS: Map<OrderKey, LimitOrder> = Map::new("bid");

    /// Plain Map with the same namespace as the `ASKS` IndexedMap primary key.
    pub const ASKS: Map<OrderKey, LimitOrder> = Map::new("ask");
}

/// Plain Maps with the new `LimitOrder` type, same namespace as the IndexedMaps.
/// Used to overwrite the old serialized values in place.
const NEW_BIDS: Map<OrderKey, perps::LimitOrder> = Map::new("bid");
const NEW_ASKS: Map<OrderKey, perps::LimitOrder> = Map::new("ask");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    let mut storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    Ok(_do_upgrade(&mut storage)?)
}

fn _do_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    let mut count = 0usize;

    // Migrate BIDS: load all old entries, convert, overwrite.
    let bids: Vec<_> = legacy::BIDS
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (key, old_order) in bids {
        let new_order = perps::LimitOrder {
            user: old_order.user,
            size: old_order.size,
            reduce_only: old_order.reduce_only,
            reserved_margin: old_order.reserved_margin,
            created_at: old_order.created_at,
            client_order_id: None,
            tp: old_order.tp,
            sl: old_order.sl,
        };
        NEW_BIDS.save(storage, key, &new_order)?;
        count += 1;
    }

    // Migrate ASKS: load all old entries, convert, overwrite.
    let asks: Vec<_> = legacy::ASKS
        .range(storage, None, None, IterationOrder::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (key, old_order) in asks {
        let new_order = perps::LimitOrder {
            user: old_order.user,
            size: old_order.size,
            reduce_only: old_order.reduce_only,
            reserved_margin: old_order.reserved_margin,
            created_at: old_order.created_at,
            client_order_id: None,
            tp: old_order.tp,
            sl: old_order.sl,
        };
        NEW_ASKS.save(storage, key, &new_order)?;
        count += 1;
    }

    tracing::info!("Migrated {count} limit orders (added client_order_id = None)");

    Ok(())
}
