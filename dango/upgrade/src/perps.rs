use {
    dango_order_book::{PairId, UsdPrice},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
    grug_types::{Addr, BlockInfo, Order, StdResult, Storage, Timestamp, addr},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Address of the oracle contract. Same on mainnet and testnet.
const ORACLE: Addr = addr!("cedc5f73cbb963a48471b849c3650e6e34cd3b6d");

/// Pre-migration `PairState`: 4 fields, no `index_price` or `last_index_time`.
mod legacy_perps {
    use {
        dango_order_book::{FundingPerUnit, FundingRate, PairId, Quantity},
        grug_storage::Map,
    };

    #[grug_types::derive(Borsh)]
    #[derive(Default)]
    pub struct PairState {
        pub long_oi: Quantity,
        pub short_oi: Quantity,
        pub funding_per_unit: FundingPerUnit,
        pub funding_rate: FundingRate,
    }

    pub const PAIR_STATES: Map<&PairId, PairState> = Map::new("pair_state");
}

pub fn do_perps_upgrades(storage: Box<dyn Storage>, block: BlockInfo) -> AppResult<()> {
    let perps_address = {
        let chain_id = CHAIN_ID.load(&storage)?;
        match chain_id.as_str() {
            MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
            TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
            _ => panic!("unknown chain id: {chain_id}"),
        }
    };

    let mut perps_storage =
        StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &perps_address]);
    let oracle_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &ORACLE]);

    do_pair_states_migration(&mut perps_storage, &oracle_storage, block.timestamp)
}

/// Migrate `PAIR_STATES` from the legacy 4-field layout to the new 6-field
/// layout that includes `index_price` and `last_index_time`.
///
/// `index_price` is seeded from the oracle contract's stored price for each
/// pair. If no oracle price exists, falls back to zero. `last_index_time` is
/// set to the current block timestamp.
fn do_pair_states_migration(
    perps_storage: &mut dyn Storage,
    oracle_storage: &dyn Storage,
    block_timestamp: Timestamp,
) -> AppResult<()> {
    let legacy_entries: Vec<(PairId, legacy_perps::PairState)> = legacy_perps::PAIR_STATES
        .range(perps_storage, None, None, Order::Ascending)
        .collect::<StdResult<_>>()?;

    for (pair_id, legacy) in legacy_entries {
        let index_price = read_oracle_price(oracle_storage, &pair_id).unwrap_or(UsdPrice::ZERO);

        let new_state = dango_types::perps::PairState {
            long_oi: legacy.long_oi,
            short_oi: legacy.short_oi,
            funding_per_unit: legacy.funding_per_unit,
            funding_rate: legacy.funding_rate,
            index_price,
            last_index_time: block_timestamp,
        };

        dango_perps::state::PAIR_STATES.save(perps_storage, &pair_id, &new_state)?;

        tracing::info!(
            %pair_id,
            %index_price,
            "Migrated PairState with index_price"
        );
    }

    Ok(())
}

/// Read the oracle contract's stored price for a pair.
///
/// Follows the same path as OracleQuerier::query_price_for_perps:
/// PRICE_SOURCES[denom] → pyth_id → PYTH_PRICES[pyth_id] → humanized_price.
fn read_oracle_price(oracle_storage: &dyn Storage, pair_id: &PairId) -> Option<UsdPrice> {
    let source = dango_oracle::PRICE_SOURCES
        .may_load(oracle_storage, pair_id)
        .ok()
        .flatten()?;

    let price = dango_oracle::PYTH_PRICES
        .may_load(oracle_storage, source.id)
        .ok()
        .flatten()?;

    Some(price.humanized_price)
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::*,
        dango_order_book::{FundingPerUnit, FundingRate, Quantity},
        dango_types::oracle::Price,
        grug_types::MockStorage,
        pyth_types::{MarketSession, constants::BTC_USD_ID},
    };

    fn btc_pair_id() -> PairId {
        "perp/btcusd".parse().unwrap()
    }

    #[test]
    fn pair_state_migration_adds_index_price() {
        let mut perps_storage = MockStorage::new();
        let mut oracle_storage = MockStorage::new();

        let pair_id = btc_pair_id();

        legacy_perps::PAIR_STATES
            .save(&mut perps_storage, &pair_id, &legacy_perps::PairState {
                long_oi: Quantity::new_int(100),
                short_oi: Quantity::new_int(50),
                funding_per_unit: FundingPerUnit::new_int(42),
                funding_rate: FundingRate::new_raw(1234),
            })
            .unwrap();

        let btc_price = UsdPrice::new_int(50_000);

        dango_oracle::PRICE_SOURCES
            .save(
                &mut oracle_storage,
                &pair_id,
                &dango_types::oracle::PriceSource {
                    id: BTC_USD_ID.id,
                    channel: BTC_USD_ID.channel,
                },
            )
            .unwrap();

        dango_oracle::PYTH_PRICES
            .save(
                &mut oracle_storage,
                BTC_USD_ID.id,
                &Price::new(
                    btc_price,
                    Timestamp::from_seconds(1_700_000_000),
                    MarketSession::Regular,
                ),
            )
            .unwrap();

        let ts = Timestamp::from_seconds(1_700_000_100);

        do_pair_states_migration(&mut perps_storage, &oracle_storage, ts).unwrap();

        let migrated = dango_perps::state::PAIR_STATES
            .load(&perps_storage, &pair_id)
            .unwrap();

        assert_eq!(migrated.long_oi, Quantity::new_int(100));
        assert_eq!(migrated.short_oi, Quantity::new_int(50));
        assert_eq!(migrated.funding_per_unit, FundingPerUnit::new_int(42));
        assert_eq!(migrated.funding_rate, FundingRate::new_raw(1234));
        assert_eq!(migrated.index_price, btc_price);
        assert_eq!(migrated.last_index_time, ts);
    }

    #[test]
    fn pair_state_migration_falls_back_to_zero_when_no_oracle() {
        let mut perps_storage = MockStorage::new();
        let oracle_storage = MockStorage::new();

        let pair_id = btc_pair_id();

        legacy_perps::PAIR_STATES
            .save(
                &mut perps_storage,
                &pair_id,
                &legacy_perps::PairState::default(),
            )
            .unwrap();

        let ts = Timestamp::from_seconds(1_700_000_000);

        do_pair_states_migration(&mut perps_storage, &oracle_storage, ts).unwrap();

        let migrated = dango_perps::state::PAIR_STATES
            .load(&perps_storage, &pair_id)
            .unwrap();

        assert_eq!(migrated.index_price, UsdPrice::ZERO);
        assert_eq!(migrated.last_index_time, ts);
    }

    #[test]
    fn empty_storage_is_noop() {
        let mut perps_storage = MockStorage::new();
        let oracle_storage = MockStorage::new();

        do_pair_states_migration(
            &mut perps_storage,
            &oracle_storage,
            Timestamp::from_seconds(0),
        )
        .unwrap();
    }
}
