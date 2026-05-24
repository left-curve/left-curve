use {
    dango_order_book::UsdPrice,
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    grug_math::{Dec128_6, Unsigned},
    grug_types::{Addr, Denom, Inner, Order, StdError, StdResult, Storage, addr},
    pyth_types::{MarketSession, PythId},
};

/// Address of the Oracle contract. Same on mainnet and testnet (per the public
/// API documentation and the live `app_config.oracle` field).
const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

/// Snapshot of the pre-refactor oracle storage shapes.
///
/// `PriceSource` was an enum with `Fixed` (test-only) and `Pyth` variants;
/// `PrecisionlessPrice` carried a `Udec128` humanized price and a phantom
/// `Undefined<Precision>` ZST. The new code collapses both into plain structs
/// holding only the fields we actually use, so the migration projects each
/// legacy entry into the new shape and re-saves it under the same storage
/// key.
mod legacy_oracle {
    use {
        grug_math::Udec128,
        grug_storage::{Map, Serde},
        grug_types::{Denom, Timestamp},
        pyth_types::{Channel, PythId},
    };

    #[grug_types::derive(Serde)]
    pub enum PriceSource {
        Fixed {
            humanized_price: Udec128,
            precision: u8,
            timestamp: Timestamp,
        },
        Pyth {
            id: PythId,
            precision: u8,
            channel: Channel,
        },
    }

    /// Borsh layout: `Udec128 (16 bytes) + Timestamp (8 bytes)`. The
    /// pre-refactor `Price<Undefined<Precision>>` ended the struct with a
    /// `Undefined<u8>` ZST that Borsh encodes as 0 bytes, so the on-disk
    /// representation matches this two-field shape verbatim.
    #[grug_types::derive(Borsh)]
    pub struct PrecisionlessPrice {
        pub humanized_price: Udec128,
        pub timestamp: Timestamp,
    }

    /// Same storage key (`"price_source"`) as `dango_oracle::PRICE_SOURCES`,
    /// so loading through this handle reads the live on-disk JSON bytes
    /// under the legacy enum shape.
    pub const PRICE_SOURCES: Map<&Denom, PriceSource, Serde> = Map::new("price_source");

    /// Same storage key (`"pyth_price"`) as `dango_oracle::PYTH_PRICES`.
    pub const PYTH_PRICES: Map<PythId, PrecisionlessPrice> = Map::new("pyth_price");
}

pub fn do_oracle_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let mut oracle_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &ORACLE]);

    do_price_sources_migration(&mut oracle_storage)?;
    do_pyth_prices_migration(&mut oracle_storage)
}

/// Re-project every `PRICE_SOURCES` entry from the legacy enum shape
/// (`{"pyth": {...}}` JSON tag) to the new struct shape (no tag).
///
/// `Fixed` variants are not expected in production but are skipped
/// defensively if encountered.
fn do_price_sources_migration(storage: &mut dyn Storage) -> AppResult<()> {
    let legacy_entries: Vec<(Denom, legacy_oracle::PriceSource)> = legacy_oracle::PRICE_SOURCES
        .range(storage, None, None, Order::Ascending)
        .collect::<StdResult<_>>()?;

    // Save the new shape under the same key. Each call to
    // `dango_oracle::PRICE_SOURCES.save` overwrites the legacy JSON bytes.
    for (denom, legacy) in legacy_entries {
        match legacy {
            legacy_oracle::PriceSource::Pyth {
                id,
                channel,
                precision: _,
            } => {
                let new_source = dango_types::oracle::PriceSource { id, channel };

                dango_oracle::PRICE_SOURCES.save(storage, &denom, &new_source)?;

                tracing::info!(%denom, %id, "Migrated price source");
            },
            legacy_oracle::PriceSource::Fixed { .. } => {
                legacy_oracle::PRICE_SOURCES.remove(storage, &denom);

                tracing::warn!(
                    %denom,
                    "Encountered legacy `Fixed` price source during migration; dropped it"
                );
            },
        }
    }

    Ok(())
}

/// Re-encode every `PYTH_PRICES` entry from the legacy Borsh layout
/// (`Udec128`-backed humanized price, 18 decimal places) to the new layout
/// (`Dec128_6`-backed `UsdPrice`, 6 decimal places).
///
/// The raw u128 of `Udec128` equals `humanized × 10^18`; the raw i128 of
/// `Dec128_6` equals `humanized × 10^6`. The conversion divides the raw
/// value by `10^12`, which `Dec128_6::checked_from_atomics(_, 18)` does
/// internally — and which truncates digits beyond 6 decimal places, exactly
/// matching the conversion that `query_price_for_perps` used to perform at
/// every query.
///
/// The new layout also carries a `market_session` field. We backfill it with
/// `Regular` for every existing entry: at the time of this migration, all
/// price sources point to crypto feeds, which trade 24/7 and are always in
/// the regular session.
fn do_pyth_prices_migration(storage: &mut dyn Storage) -> AppResult<()> {
    let legacy_entries: Vec<(PythId, legacy_oracle::PrecisionlessPrice)> =
        legacy_oracle::PYTH_PRICES
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<_>>()?;

    for (id, legacy) in legacy_entries {
        // `Udec128 (Dec<u128, 18>)` → `Dec<i128, 18>` → `Dec128_6` (raw / 10^12).
        let signed = legacy
            .humanized_price
            .checked_into_signed()
            .map_err(StdError::from)?;
        let new_inner =
            Dec128_6::checked_from_atomics(signed.into_inner(), 18).map_err(StdError::from)?;
        let new_price = dango_types::oracle::Price {
            humanized_price: UsdPrice::new(new_inner),
            timestamp: legacy.timestamp,
            market_session: MarketSession::Regular,
        };

        dango_oracle::PYTH_PRICES.save(storage, id, &new_price)?;

        tracing::info!(
            %id,
            humanized_price = %new_price.humanized_price,
            timestamp = new_price.timestamp.to_rfc3339_string(),
            "Migrated Pyth price"
        );
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_types::constants::{atom, btc, eth, usdc},
        grug_math::Udec128,
        grug_types::{MockStorage, Timestamp, btree_map},
        pyth_types::constants::{ATOM_USD_ID, BTC_USD_ID, ETH_USD_ID, USDC_USD_ID},
        std::str::FromStr,
    };

    /// A representative pre-refactor `PRICE_SOURCES` map that mirrors what is
    /// actually live on mainnet today (Pyth-only, with `precision` set per
    /// the values in `dango/types/src/constants/pyth.rs`).
    fn seed_legacy_price_sources(storage: &mut dyn Storage) {
        let entries = btree_map! {
            atom::DENOM.clone() => legacy_oracle::PriceSource::Pyth {
                id: ATOM_USD_ID.id,
                channel: ATOM_USD_ID.channel,
                precision: 6,
            },
            btc::DENOM.clone() => legacy_oracle::PriceSource::Pyth {
                id: BTC_USD_ID.id,
                channel: BTC_USD_ID.channel,
                precision: 8,
            },
            eth::DENOM.clone() => legacy_oracle::PriceSource::Pyth {
                id: ETH_USD_ID.id,
                channel: ETH_USD_ID.channel,
                precision: 18,
            },
            usdc::DENOM.clone() => legacy_oracle::PriceSource::Pyth {
                id: USDC_USD_ID.id,
                channel: USDC_USD_ID.channel,
                precision: 6,
            },
        };
        for (denom, source) in entries {
            legacy_oracle::PRICE_SOURCES
                .save(storage, &denom, &source)
                .unwrap();
        }
    }

    #[test]
    fn price_sources_migration_projects_pyth_entries_and_drops_precision() {
        let mut storage = MockStorage::new();
        seed_legacy_price_sources(&mut storage);

        do_price_sources_migration(&mut storage).unwrap();

        // Each entry should now decode under the new struct shape with the
        // `id` and `channel` preserved; `precision` is gone.
        let migrated_btc = dango_oracle::PRICE_SOURCES
            .load(&storage, &btc::DENOM)
            .unwrap();
        assert_eq!(migrated_btc.id, BTC_USD_ID.id);
        assert_eq!(migrated_btc.channel, BTC_USD_ID.channel);

        let migrated_eth = dango_oracle::PRICE_SOURCES
            .load(&storage, &eth::DENOM)
            .unwrap();
        assert_eq!(migrated_eth.id, ETH_USD_ID.id);
        assert_eq!(migrated_eth.channel, ETH_USD_ID.channel);

        // Every seeded denom must survive the migration.
        let count = dango_oracle::PRICE_SOURCES
            .range(&storage, None, None, Order::Ascending)
            .count();
        assert_eq!(count, 4);
    }

    #[test]
    fn price_sources_migration_drops_legacy_fixed_entries() {
        let mut storage = MockStorage::new();

        legacy_oracle::PRICE_SOURCES
            .save(
                &mut storage,
                &btc::DENOM,
                &legacy_oracle::PriceSource::Pyth {
                    id: BTC_USD_ID.id,
                    channel: BTC_USD_ID.channel,
                    precision: 8,
                },
            )
            .unwrap();
        legacy_oracle::PRICE_SOURCES
            .save(
                &mut storage,
                &eth::DENOM,
                &legacy_oracle::PriceSource::Fixed {
                    humanized_price: Udec128::new(2_000),
                    precision: 18,
                    timestamp: Timestamp::from_seconds(0),
                },
            )
            .unwrap();

        do_price_sources_migration(&mut storage).unwrap();

        // BTC survives as the new struct.
        assert!(
            dango_oracle::PRICE_SOURCES
                .may_load(&storage, &btc::DENOM)
                .unwrap()
                .is_some(),
        );
        // ETH (the Fixed entry) is dropped entirely.
        assert!(
            dango_oracle::PRICE_SOURCES
                .may_load(&storage, &eth::DENOM)
                .unwrap()
                .is_none(),
        );
    }

    #[test]
    fn pyth_prices_migration_rescales_humanized_price() {
        let mut storage = MockStorage::new();

        // Seed three legacy entries with prices that span integer and
        // sub-cent humanized values so the truncation behavior is exercised.
        legacy_oracle::PYTH_PRICES
            .save(
                &mut storage,
                BTC_USD_ID.id,
                &legacy_oracle::PrecisionlessPrice {
                    // $50_000.00
                    humanized_price: Udec128::new(50_000),
                    timestamp: Timestamp::from_seconds(1_700_000_000),
                },
            )
            .unwrap();
        legacy_oracle::PYTH_PRICES
            .save(
                &mut storage,
                ETH_USD_ID.id,
                &legacy_oracle::PrecisionlessPrice {
                    // $2_000.50
                    humanized_price: Udec128::from_str("2000.50").unwrap(),
                    timestamp: Timestamp::from_seconds(1_700_000_001),
                },
            )
            .unwrap();
        legacy_oracle::PYTH_PRICES
            .save(
                &mut storage,
                USDC_USD_ID.id,
                &legacy_oracle::PrecisionlessPrice {
                    // $1.000000000123456789 — has 12 digits past the 6-decimal
                    // cutoff; should truncate cleanly to "1.000000".
                    humanized_price: Udec128::from_str("1.000000000123456789").unwrap(),
                    timestamp: Timestamp::from_seconds(1_700_000_002),
                },
            )
            .unwrap();

        do_pyth_prices_migration(&mut storage).unwrap();

        let btc = dango_oracle::PYTH_PRICES
            .load(&storage, BTC_USD_ID.id)
            .unwrap();
        assert_eq!(
            btc.humanized_price,
            UsdPrice::new(Dec128_6::from_str("50000").unwrap()),
        );
        assert_eq!(btc.timestamp, Timestamp::from_seconds(1_700_000_000));
        assert_eq!(btc.market_session, MarketSession::Regular);

        let eth = dango_oracle::PYTH_PRICES
            .load(&storage, ETH_USD_ID.id)
            .unwrap();
        assert_eq!(
            eth.humanized_price,
            UsdPrice::new(Dec128_6::from_str("2000.5").unwrap()),
        );
        assert_eq!(eth.market_session, MarketSession::Regular);

        let usdc = dango_oracle::PYTH_PRICES
            .load(&storage, USDC_USD_ID.id)
            .unwrap();
        assert_eq!(
            usdc.humanized_price,
            UsdPrice::new(Dec128_6::from_str("1.000000").unwrap()),
        );
        assert_eq!(usdc.market_session, MarketSession::Regular);
    }

    #[test]
    fn empty_substore_is_noop() {
        let mut storage = MockStorage::new();

        do_price_sources_migration(&mut storage).unwrap();
        do_pyth_prices_migration(&mut storage).unwrap();

        // Both maps stay empty.
        assert_eq!(
            dango_oracle::PRICE_SOURCES
                .range(&storage, None, None, Order::Ascending)
                .count(),
            0,
        );
        assert_eq!(
            dango_oracle::PYTH_PRICES
                .range(&storage, None, None, Order::Ascending)
                .count(),
            0,
        );
    }
}
