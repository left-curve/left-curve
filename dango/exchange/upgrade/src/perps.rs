use {
    dango_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    dango_order_book::PairId,
    dango_perps::state::PAIR_STATES,
    dango_primitives::{Addr, Order, StdResult, Storage, addr},
    dango_storage::Map,
    dango_types::perps::PairState,
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Pre-migration perps storage shapes.
mod legacy_perps {
    use {
        dango_order_book::{FundingPerUnit, FundingRate, Quantity, UsdPrice},
        dango_primitives::Timestamp,
    };

    /// `PairState` as stored before the `oracle_price` / `last_oracle_time`
    /// fields were appended. Field order and types must match the old on-disk
    /// Borsh layout exactly.
    #[dango_primitives::derive(Serde, Borsh)]
    #[derive(Default)]
    pub struct PairState {
        pub long_oi: Quantity,
        pub short_oi: Quantity,
        pub funding_per_unit: FundingPerUnit,
        pub funding_rate: FundingRate,
        pub index_price: UsdPrice,
        pub last_index_time: Timestamp,
    }
}

/// Reads the pre-upgrade `PairState`s, keyed identically to the live map.
const LEGACY_PAIR_STATES: Map<&PairId, legacy_perps::PairState> = Map::new("pair_state");

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

    migrate_pair_states(&mut perps_storage)?;

    Ok(())
}

/// Append `oracle_price` / `last_oracle_time` to every stored `PairState`,
/// seeding both from the pre-upgrade mark (`index_price` / `last_index_time`).
///
/// The upgrade must activate with a fresh index refresh (or while user actions
/// are paused, then refreshed), so at activation the mark equals the true
/// external oracle price and the seeded reference is not a closed-session
/// drifted value.
fn migrate_pair_states(perps_storage: &mut dyn Storage) -> StdResult<()> {
    // Collect before writing: the legacy and new maps share the "pair_state"
    // key, so each save overwrites an entry we would otherwise still be
    // iterating.
    let legacy = LEGACY_PAIR_STATES
        .range(perps_storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (pair_id, old) in legacy {
        let migrated = PairState {
            long_oi: old.long_oi,
            short_oi: old.short_oi,
            funding_per_unit: old.funding_per_unit,
            funding_rate: old.funding_rate,
            index_price: old.index_price,
            last_index_time: old.last_index_time,
            oracle_price: old.index_price,
            last_oracle_time: old.last_index_time,
        };

        PAIR_STATES.save(perps_storage, &pair_id, &migrated)?;
    }

    tracing::info!("migrated perps PairState: seeded oracle_price/last_oracle_time from the mark");

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::{FundingPerUnit, Quantity, UsdPrice},
        dango_primitives::{MockStorage, Timestamp},
    };

    #[test]
    fn pair_state_migration_seeds_oracle_from_mark() {
        let mut storage = MockStorage::new();
        let pair_id: PairId = "perp/btcusd".parse().unwrap();

        let legacy = legacy_perps::PairState {
            long_oi: Quantity::new_int(3),
            short_oi: Quantity::new_int(2),
            funding_per_unit: FundingPerUnit::new_int(7),
            index_price: UsdPrice::new_int(60_000),
            last_index_time: Timestamp::from_seconds(1_700_000_000),
            ..Default::default()
        };
        LEGACY_PAIR_STATES
            .save(&mut storage, &pair_id, &legacy)
            .unwrap();

        migrate_pair_states(&mut storage).unwrap();

        let migrated = PAIR_STATES.load(&storage, &pair_id).unwrap();

        // Pre-existing fields are preserved byte-for-byte.
        assert_eq!(migrated.long_oi, legacy.long_oi);
        assert_eq!(migrated.short_oi, legacy.short_oi);
        assert_eq!(migrated.funding_per_unit, legacy.funding_per_unit);
        assert_eq!(migrated.index_price, legacy.index_price);
        assert_eq!(migrated.last_index_time, legacy.last_index_time);

        // The new reference fields are seeded from the mark.
        assert_eq!(migrated.oracle_price, legacy.index_price);
        assert_eq!(migrated.last_oracle_time, legacy.last_index_time);
    }
}
