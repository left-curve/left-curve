use {
    dango_types::oracle::PriceSourceWithWeight,
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

    migrate_price_sources(&mut oracle_storage)?;

    Ok(())
}

/// Migrate the oracle's price sources from a single source per denom to a
/// weighted list of sources per denom.
///
/// Every existing source is wrapped into a one-element list carrying the full
/// weight (one). The combined price of a one-element list is just that source's
/// price, so this preserves the pre-upgrade behavior exactly. The legacy and new
/// maps share the same storage key (`"price_source"`), so we collect every
/// legacy entry first, then overwrite each in place.
fn migrate_price_sources(storage: &mut dyn Storage) -> AppResult<()> {
    let legacy = legacy_oracle::PRICE_SOURCES
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (denom, price_source) in legacy {
        let weighted = vec![PriceSourceWithWeight::single(price_source)];
        dango_oracle::PRICE_SOURCES.save(storage, &denom, &weighted)?;

        tracing::info!(%denom, "migrated oracle price source to a weighted list");
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
    fn migrating_price_sources_wraps_each_into_a_weighted_singleton() {
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

        // Each denom now maps to a one-element weighted list (weight 1) holding
        // the original source unchanged.
        assert_eq!(
            dango_oracle::PRICE_SOURCES.load(&storage, &btc).unwrap(),
            vec![PriceSourceWithWeight::single(PriceSource {
                id: 1,
                channel: Channel::RealTime,
            })],
        );
        assert_eq!(
            dango_oracle::PRICE_SOURCES.load(&storage, &eth).unwrap(),
            vec![PriceSourceWithWeight::single(PriceSource {
                id: 2,
                channel: Channel::RealTime,
            })],
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
