use {
    grug::{Addr, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
};

/// Address of the Bank contract. Same across mainnet and testnet.
const BANK: Addr = addr!("e0b49f70991ecab05d5d7dc1f71e4ede63c8f2b7");

/// Address of the Gateway contract. Same across mainnet and testnet.
const GATEWAY: Addr = addr!("c51e2cbe9636a90c86463ac3eb18fbee92b700d1");

mod legacy_gateway {
    use grug::{Denom, Map, Uint128};

    /// Storage key the rate-limit-hardening release used for the per-denom
    /// draining outbound cap. The rolling-window release replaces it with
    /// `dango_gateway::SUPPLY_SNAPSHOTS`, so the migration drops every entry
    /// behind this prefix.
    pub const OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");
}

pub fn do_gateway_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    let bank_storage = StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &BANK]);
    let mut gateway_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &GATEWAY]);

    do_gateway_rolling_window_seed(&mut gateway_storage, &bank_storage)
}

/// Drop the old draining-quota state and seed a supply snapshot for every
/// rate-limited denom so the rolling-window cap is enforced from the very
/// next withdraw, without waiting for the first cron tick after the
/// upgrade.
fn do_gateway_rolling_window_seed(
    gateway_storage: &mut dyn Storage,
    bank_storage: &dyn Storage,
) -> AppResult<()> {
    // Wipe every `OUTBOUND_QUOTAS` entry. The rolling-window contract
    // doesn't read this prefix; leaving it behind would just be dead state.
    legacy_gateway::OUTBOUND_QUOTAS.clear(gateway_storage, None, None);

    for denom in dango_gateway::RATE_LIMITS.load(gateway_storage)?.keys() {
        let supply = dango_bank::SUPPLIES.load(bank_storage, denom)?;
        dango_gateway::SUPPLY_SNAPSHOTS.save(gateway_storage, denom, &supply)?;

        tracing::info!(%denom, %supply, "Seeded gateway supply snapshots");
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::{do_gateway_rolling_window_seed, legacy_gateway::OUTBOUND_QUOTAS},
        dango_bank::SUPPLIES,
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

        OUTBOUND_QUOTAS
            .save(&mut gateway, &usdc, &Uint128::new(123))
            .unwrap();
        OUTBOUND_QUOTAS
            .save(&mut gateway, &eth, &Uint128::new(456))
            .unwrap();

        // Bank state mirrored from the bank contract's `supply` map.
        SUPPLIES
            .save(&mut bank, &usdc, &Uint128::new(100_000_000))
            .unwrap();
        SUPPLIES
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
        assert!(OUTBOUND_QUOTAS.may_load(&gateway, &usdc).unwrap().is_none());
        assert!(OUTBOUND_QUOTAS.may_load(&gateway, &eth).unwrap().is_none());
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
