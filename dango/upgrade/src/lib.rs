use {
    dango_gateway::{RATE_LIMITS, SUPPLY_SNAPSHOTS},
    dango_types::config::AppConfig,
    grug::{BlockInfo, Denom, JsonDeExt, Map, StdResult, Storage, Uint128},
    grug_app::{APP_CONFIG, AppResult, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
};

/// Storage key the rate-limit-hardening release used for the per-denom
/// draining outbound cap. The rolling-window release replaces it with
/// `SUPPLY_SNAPSHOTS`, so the migration drops every entry behind this
/// prefix.
const OLD_OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");

/// Mirror of the bank contract's supply storage so the migration can read
/// the current supply per denom without going through the bank's query
/// path (which isn't available from the migration context).
const BANK_SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // Look up the bank and gateway addresses for the gateway rolling-window
    // migration. Reading from on-chain config keeps the migration portable
    // across mainnet, testnet, and devnet without a per-chain address table.
    let chain_config = CONFIG.load(&storage)?;
    let app_config: AppConfig = APP_CONFIG.load(&storage)?.deserialize_json()?;
    let bank_address = chain_config.bank;
    let gateway_address = app_config.addresses.gateway;

    // `Storage` is `DynClone`, so `Box<dyn Storage>` is `Clone`. Each clone
    // points at the same backing KV store; writes through any provider land
    // in the same place.
    let mut gateway_storage =
        StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &gateway_address]);
    let bank_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &bank_address]);

    do_gateway_rolling_window_seed(&mut gateway_storage, &bank_storage)?;

    Ok(())
}

/// Drop the old draining-quota state and seed a supply snapshot for every
/// rate-limited denom so the rolling-window cap is enforced from the very
/// next withdraw, without waiting for the first cron tick after the
/// upgrade.
fn do_gateway_rolling_window_seed(
    gateway_storage: &mut dyn Storage,
    bank_storage: &dyn Storage,
) -> StdResult<()> {
    // Wipe every `OUTBOUND_QUOTAS` entry. The rolling-window contract
    // doesn't read this prefix; leaving it behind would just be dead state.
    OLD_OUTBOUND_QUOTAS.clear(gateway_storage, None, None);

    let mut seeded = 0usize;
    for denom in RATE_LIMITS.load(gateway_storage)?.keys() {
        let supply = BANK_SUPPLIES.load(bank_storage, denom)?;
        SUPPLY_SNAPSHOTS.save(gateway_storage, denom, &supply)?;
        seeded += 1;
    }

    tracing::info!("Seeded gateway supply snapshots for {seeded} denom(s)");

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    mod gateway_rolling_window {
        use {
            super::super::{BANK_SUPPLIES, OLD_OUTBOUND_QUOTAS, do_gateway_rolling_window_seed},
            dango_gateway::{RATE_LIMITS, SUPPLY_SNAPSHOTS},
            dango_types::gateway::RateLimit,
            grug::{Denom, MockStorage, Udec128, Uint128, btree_map},
            std::str::FromStr,
        };

        fn denom(s: &str) -> Denom {
            Denom::from_str(s).unwrap()
        }

        #[test]
        fn seeds_snapshots_and_wipes_old_quotas() {
            let mut gateway = MockStorage::new();
            let mut bank = MockStorage::new();

            let usdc = denom("bridge/usdc");
            let eth = denom("bridge/eth");

            // Pre-upgrade state: configured rate limits plus drained
            // OUTBOUND_QUOTAS that the rolling-window release shouldn't read.
            RATE_LIMITS
                .save(&mut gateway, &btree_map! {
                    usdc.clone() => RateLimit::new_unchecked(Udec128::new_percent(10)),
                    eth.clone()  => RateLimit::new_unchecked(Udec128::new_percent(20)),
                })
                .unwrap();

            OLD_OUTBOUND_QUOTAS
                .save(&mut gateway, &usdc, &Uint128::new(123))
                .unwrap();
            OLD_OUTBOUND_QUOTAS
                .save(&mut gateway, &eth, &Uint128::new(456))
                .unwrap();

            // Bank state mirrored from the bank contract's `supply` map.
            BANK_SUPPLIES
                .save(&mut bank, &usdc, &Uint128::new(100_000_000))
                .unwrap();
            BANK_SUPPLIES
                .save(&mut bank, &eth, &Uint128::new(50_000_000))
                .unwrap();

            do_gateway_rolling_window_seed(&mut gateway, &bank).unwrap();

            // Snapshots match the bank's current supply for every
            // rate-limited denom.
            assert_eq!(
                SUPPLY_SNAPSHOTS.load(&gateway, &usdc).unwrap(),
                Uint128::new(100_000_000),
            );
            assert_eq!(
                SUPPLY_SNAPSHOTS.load(&gateway, &eth).unwrap(),
                Uint128::new(50_000_000),
            );

            // The legacy OUTBOUND_QUOTAS map is empty.
            assert!(
                OLD_OUTBOUND_QUOTAS
                    .may_load(&gateway, &usdc)
                    .unwrap()
                    .is_none()
            );
            assert!(
                OLD_OUTBOUND_QUOTAS
                    .may_load(&gateway, &eth)
                    .unwrap()
                    .is_none()
            );
        }

        /// A chain with no rate limits at all is a no-op: nothing reads,
        /// nothing writes, and OUTBOUND_QUOTAS stays empty.
        #[test]
        fn empty_rate_limits_is_noop() {
            let mut gateway = MockStorage::new();
            let bank = MockStorage::new();

            RATE_LIMITS.save(&mut gateway, &btree_map! {}).unwrap();

            do_gateway_rolling_window_seed(&mut gateway, &bank).unwrap();

            assert_eq!(
                SUPPLY_SNAPSHOTS
                    .range(&gateway, None, None, grug::Order::Ascending)
                    .count(),
                0,
            );
        }
    }
}
