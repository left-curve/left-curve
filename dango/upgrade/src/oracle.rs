use {
    dango_types::oracle::PriceConfig,
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    grug_types::{Addr, Order, StdResult, Storage, addr},
};

/// Address of the Oracle contract. Same on mainnet and testnet.
const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

/// Pre-migration oracle storage shapes.
mod legacy_oracle {
    use {
        dango_types::oracle::PriceSource,
        grug_storage::{Map, Serde},
        grug_types::Denom,
    };

    /// Before the upgrade, each denom mapped to exactly one price source.
    pub const PRICE_SOURCES: Map<&Denom, PriceSource, Serde> = Map::new("price_source");
}

pub fn do_oracle_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let mut oracle_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &ORACLE]);

    migrate_price_sources(&mut oracle_storage)
}

/// Migrate the oracle's price sources from a single source per denom to the new
/// [`PriceConfig`] shape.
///
/// Every existing source becomes `PriceConfig::Single`, which prices identically,
/// so this preserves pre-upgrade behavior exactly. The legacy and new maps share
/// the same storage key (`"price_source"`), so we collect every legacy entry
/// first, then overwrite each in place.
fn migrate_price_sources(storage: &mut dyn Storage) -> AppResult<()> {
    // Skip if the data is already in the new `PriceConfig` shape — e.g. a chain
    // genesis'd on this version. Those values don't decode as the legacy
    // `PriceSource`, so rather than swallowing that decode error below we detect
    // "already migrated" up front: the first entry decodes cleanly under the new
    // shape.
    if dango_oracle::PRICE_SOURCES
        .range(storage, None, None, Order::Ascending)
        .next()
        .is_some_and(|res| res.is_ok())
    {
        return Ok(());
    }

    // Old format: collect every legacy entry (propagating a genuine decode
    // error), then wrap each into a `Single` config — identical pricing behavior.
    let legacy = legacy_oracle::PRICE_SOURCES
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (denom, source) in legacy {
        dango_oracle::PRICE_SOURCES.save(storage, &denom, &PriceConfig::Single(source))?;
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::oracle::PriceSource,
        grug_types::{Denom, MockStorage},
        pyth_types::Channel,
        std::str::FromStr,
    };

    #[test]
    fn migrating_wraps_each_source_into_a_single_config() {
        let mut storage = MockStorage::default();

        let btc = Denom::from_str("perp/btcusd").unwrap();
        let eth = Denom::from_str("eth").unwrap();

        // Seed the pre-upgrade single price sources.
        legacy_oracle::PRICE_SOURCES
            .save(&mut storage, &btc, &PriceSource {
                id: 1,
                channel: Channel::RealTime,
            })
            .unwrap();
        legacy_oracle::PRICE_SOURCES
            .save(&mut storage, &eth, &PriceSource {
                id: 2,
                channel: Channel::RealTime,
            })
            .unwrap();

        migrate_price_sources(&mut storage).unwrap();

        // Each denom now maps to a `Single` config holding the original source.
        assert_eq!(
            dango_oracle::PRICE_SOURCES.load(&storage, &btc).unwrap(),
            PriceConfig::Single(PriceSource {
                id: 1,
                channel: Channel::RealTime,
            }),
        );
        assert_eq!(
            dango_oracle::PRICE_SOURCES.load(&storage, &eth).unwrap(),
            PriceConfig::Single(PriceSource {
                id: 2,
                channel: Channel::RealTime,
            }),
        );
    }

    #[test]
    fn migrating_empty_storage_is_a_noop() {
        let mut storage = MockStorage::default();

        migrate_price_sources(&mut storage).unwrap();

        assert!(
            dango_oracle::PRICE_SOURCES
                .range(&storage, None, None, Order::Ascending)
                .next()
                .is_none()
        );
    }
}
