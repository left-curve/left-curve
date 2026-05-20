use {
    grug::{Addr, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
};

/// Address of the Taxman contract. Same on mainnet and testnet, verified via
/// the public GraphQL API.
const TAXMAN: Addr = addr!("da70a9c1417aee00f960fe896add9d571f9c365b");

mod legacy_taxman {
    use {
        dango_types::account_factory::UserIndex,
        grug::{Map, Timestamp, Udec128_6},
    };

    /// Cumulative spot-DEX trading volume that the now-deleted
    /// `taxman::ExecuteMsg::ReportVolumes` handler used to write. With the
    /// spot DEX gone there is no writer, so the migration drops every entry
    /// behind this prefix.
    pub const VOLUMES_BY_USER: Map<(UserIndex, Timestamp), Udec128_6> = Map::new("volume__user");
}

pub fn do_taxman_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let mut taxman_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &TAXMAN]);

    do_volumes_by_user_clear(&mut taxman_storage)
}

/// Drop every entry behind the legacy `VOLUMES_BY_USER` prefix. After the
/// spot-DEX retirement these records are unreachable: the only writer was
/// removed alongside the source-level storage handle, and the perps contract
/// tracks volume in its own substore.
fn do_volumes_by_user_clear(taxman_storage: &mut dyn Storage) -> AppResult<()> {
    legacy_taxman::VOLUMES_BY_USER.clear(taxman_storage, None, None);

    tracing::info!("Cleared taxman `VOLUMES_BY_USER` (legacy spot-DEX volume records)");

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::{do_volumes_by_user_clear, legacy_taxman::VOLUMES_BY_USER},
        grug::{MockStorage, Order, Timestamp, Udec128_6},
    };

    #[test]
    fn wipes_every_legacy_volume_entry() {
        let mut storage = MockStorage::new();

        // Seed a handful of entries spread across users and days so the
        // assertion below catches both partial and trailing leftovers.
        VOLUMES_BY_USER
            .save(
                &mut storage,
                (0, Timestamp::from_seconds(86_400)),
                &Udec128_6::new(1),
            )
            .unwrap();
        VOLUMES_BY_USER
            .save(
                &mut storage,
                (0, Timestamp::from_seconds(172_800)),
                &Udec128_6::new(2),
            )
            .unwrap();
        VOLUMES_BY_USER
            .save(
                &mut storage,
                (42, Timestamp::from_seconds(86_400)),
                &Udec128_6::new(3),
            )
            .unwrap();

        do_volumes_by_user_clear(&mut storage).unwrap();

        assert_eq!(
            VOLUMES_BY_USER
                .range(&storage, None, None, Order::Ascending)
                .count(),
            0,
        );
    }

    /// A taxman substore that never recorded any volumes (the only state the
    /// chain will be in once the upgrade has run once and immediately again
    /// in a backfill) must remain valid.
    #[test]
    fn empty_substore_is_noop() {
        let mut storage = MockStorage::new();

        do_volumes_by_user_clear(&mut storage).unwrap();

        assert_eq!(
            VOLUMES_BY_USER
                .range(&storage, None, None, Order::Ascending)
                .count(),
            0,
        );
    }
}
