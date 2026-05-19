use {
    dango_order_book::{Dimensionless, UsdValue},
    dango_perps::state::PARAM,
    dango_types::{
        config::AppConfig,
        perps::{Param, RateSchedule},
    },
    grug::{Addr, BlockInfo, Duration, Item, JsonDeExt, StdResult, Storage, addr},
    grug_app::{APP_CONFIG, AppResult, CHAIN_ID, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
};

mod legacy_gateway {
    use grug::{Denom, Map, Uint128};

    /// Storage key the rate-limit-hardening release used for the per-denom
    /// draining outbound cap. The rolling-window release replaces it with
    /// `dango_gateway::SUPPLY_SNAPSHOTS`, so the migration drops every entry
    /// behind this prefix.
    pub const OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");
}

/// Frozen historical schemas, kept around so migrations can decode pre-
/// upgrade storage bytes. Anything in here represents on-chain layout at
/// some past point in time — do not re-order, rename, or change field
/// types of these structs.
mod legacy_perps {
    use {
        super::{Dimensionless, Duration, Item, RateSchedule, UsdValue},
        std::option::Option,
    };

    /// `Param` schema as it existed before `min_liquidation_value` was
    /// introduced.
    #[grug::derive(Borsh)]
    pub struct LegacyParam {
        pub max_unlocks: usize,
        pub max_open_orders: usize,
        pub maker_fee_rates: RateSchedule,
        pub taker_fee_rates: RateSchedule,
        pub protocol_fee_rate: Dimensionless,
        pub liquidation_fee_rate: Dimensionless,
        pub liquidation_buffer_ratio: Dimensionless,
        pub funding_period: Duration,
        pub vault_total_weight: Dimensionless,
        pub vault_cooldown_period: Duration,
        pub referral_active: bool,
        pub min_referrer_volume: UsdValue,
        pub referrer_commission_rates: RateSchedule,
        pub vault_deposit_cap: Option<UsdValue>,
        pub max_action_batch_size: usize,
    }

    pub const LEGACY_PARAM: Item<LegacyParam> = Item::new("param");
}

/// Floor backfilled into `Param.min_liquidation_value` on upgrade. Mirrors
/// the genesis value so post-upgrade chains match a fresh chain.
const MIGRATED_MIN_LIQUIDATION_VALUE: UsdValue = UsdValue::new_int(10);

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
    let bank_storage = StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &bank_address]);

    do_gateway_rolling_window_seed(&mut gateway_storage, &bank_storage)?;

    do_perps_upgrades(storage)?;

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
    legacy_gateway::OUTBOUND_QUOTAS.clear(gateway_storage, None, None);

    for denom in dango_gateway::RATE_LIMITS.load(gateway_storage)?.keys() {
        let supply = dango_bank::SUPPLIES.load(bank_storage, denom)?;
        dango_gateway::SUPPLY_SNAPSHOTS.save(gateway_storage, denom, &supply)?;

        tracing::info!(%denom, %supply, "Seeded gateway supply snapshots");
    }

    Ok(())
}

// Unused in the current upgrade.
fn do_perps_upgrades(storage: Box<dyn Storage>) -> AppResult<()> {
    const MAINNET_CHAIN_ID: &str = "dango-1";
    const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

    const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
    const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

    // Find the address of the perps contract corresponding to the current chain.
    let perps_address = {
        let chain_id = CHAIN_ID.load(&storage)?;
        match chain_id.as_str() {
            MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
            TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
            _ => panic!("unknown chain id: {chain_id}"),
        }
    };

    // Create the prefixed storage for the perps contract.
    let mut perps_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    do_min_liquidation_value_backfill(&mut perps_storage)?;

    Ok(())
}

/// Read the legacy `Param` (without `min_liquidation_value`) and re-save it
/// under the new schema with the floor set to
/// `MIGRATED_MIN_LIQUIDATION_VALUE`. Idempotent: if the new schema already
/// decodes, the migration short-circuits.
///
/// Note: Borsh strict-decoding makes the two schemas mutually exclusive —
/// legacy bytes fail to decode as the new `Param` (missing trailing field),
/// and new bytes fail to decode as `LegacyParam` (trailing bytes). So
/// `PARAM.load(...).is_ok()` is a reliable "already migrated" signal.
fn do_min_liquidation_value_backfill(storage: &mut dyn Storage) -> StdResult<()> {
    if PARAM.load(storage).is_ok() {
        tracing::info!("`Param` already in new schema; skipping min_liquidation_value backfill");
        return Ok(());
    }

    let legacy = legacy_perps::LEGACY_PARAM.load(storage)?;

    let new_param = Param {
        max_unlocks: legacy.max_unlocks,
        max_open_orders: legacy.max_open_orders,
        maker_fee_rates: legacy.maker_fee_rates,
        taker_fee_rates: legacy.taker_fee_rates,
        protocol_fee_rate: legacy.protocol_fee_rate,
        liquidation_fee_rate: legacy.liquidation_fee_rate,
        liquidation_buffer_ratio: legacy.liquidation_buffer_ratio,
        funding_period: legacy.funding_period,
        vault_total_weight: legacy.vault_total_weight,
        vault_cooldown_period: legacy.vault_cooldown_period,
        referral_active: legacy.referral_active,
        min_referrer_volume: legacy.min_referrer_volume,
        referrer_commission_rates: legacy.referrer_commission_rates,
        vault_deposit_cap: legacy.vault_deposit_cap,
        max_action_batch_size: legacy.max_action_batch_size,
        min_liquidation_value: MIGRATED_MIN_LIQUIDATION_VALUE,
    };

    PARAM.save(storage, &new_param)?;

    tracing::info!(
        min_liquidation_value = %MIGRATED_MIN_LIQUIDATION_VALUE,
        "Backfilled `Param.min_liquidation_value`"
    );

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {
        super::{
            MIGRATED_MIN_LIQUIDATION_VALUE, do_min_liquidation_value_backfill,
            legacy_perps::{LEGACY_PARAM, LegacyParam},
        },
        dango_order_book::{Dimensionless, UsdValue},
        dango_perps::state::PARAM,
        dango_types::perps::RateSchedule,
        grug::{Duration, MockStorage},
    };

    /// A `LegacyParam` with non-default field values, used to assert the
    /// migration carries every field across unchanged.
    fn populated_legacy_param() -> LegacyParam {
        LegacyParam {
            max_unlocks: 10,
            max_open_orders: 100,
            maker_fee_rates: RateSchedule {
                base: Dimensionless::new_permille(1),
                ..Default::default()
            },
            taker_fee_rates: RateSchedule {
                base: Dimensionless::new_permille(2),
                ..Default::default()
            },
            protocol_fee_rate: Dimensionless::new_permille(200),
            liquidation_fee_rate: Dimensionless::new_permille(10),
            liquidation_buffer_ratio: Dimensionless::new_permille(50),
            funding_period: Duration::from_hours(1),
            vault_total_weight: Dimensionless::new_int(7),
            vault_cooldown_period: Duration::from_days(1),
            referral_active: true,
            min_referrer_volume: UsdValue::new_int(1_000),
            referrer_commission_rates: RateSchedule {
                base: Dimensionless::new_permille(5),
                ..Default::default()
            },
            vault_deposit_cap: Some(UsdValue::new_int(1_000_000)),
            max_action_batch_size: 5,
        }
    }

    /// Migration loads the legacy `Param`, re-saves it under the new
    /// schema with `min_liquidation_value` set to the configured floor,
    /// and preserves every other field.
    #[test]
    fn min_liquidation_value_backfill_populates_field() {
        let mut storage = MockStorage::new();
        let legacy = populated_legacy_param();
        LEGACY_PARAM.save(&mut storage, &legacy).unwrap();

        do_min_liquidation_value_backfill(&mut storage).unwrap();

        let migrated = PARAM.load(&storage).unwrap();
        assert_eq!(
            migrated.min_liquidation_value,
            MIGRATED_MIN_LIQUIDATION_VALUE
        );

        // Every other field is carried across unchanged.
        assert_eq!(migrated.max_unlocks, legacy.max_unlocks);
        assert_eq!(migrated.max_open_orders, legacy.max_open_orders);
        assert_eq!(migrated.maker_fee_rates, legacy.maker_fee_rates);
        assert_eq!(migrated.taker_fee_rates, legacy.taker_fee_rates);
        assert_eq!(migrated.protocol_fee_rate, legacy.protocol_fee_rate);
        assert_eq!(migrated.liquidation_fee_rate, legacy.liquidation_fee_rate);
        assert_eq!(
            migrated.liquidation_buffer_ratio,
            legacy.liquidation_buffer_ratio
        );
        assert_eq!(migrated.funding_period, legacy.funding_period);
        assert_eq!(migrated.vault_total_weight, legacy.vault_total_weight);
        assert_eq!(migrated.vault_cooldown_period, legacy.vault_cooldown_period);
        assert_eq!(migrated.referral_active, legacy.referral_active);
        assert_eq!(migrated.min_referrer_volume, legacy.min_referrer_volume);
        assert_eq!(
            migrated.referrer_commission_rates,
            legacy.referrer_commission_rates
        );
        assert_eq!(migrated.vault_deposit_cap, legacy.vault_deposit_cap);
        assert_eq!(migrated.max_action_batch_size, legacy.max_action_batch_size);
    }

    /// Running the migration twice yields the same `Param` as running it
    /// once: the second pass short-circuits via the `load` guard, so a
    /// hand-tweaked floor is not silently reset.
    #[test]
    fn min_liquidation_value_backfill_idempotent() {
        let mut storage = MockStorage::new();
        LEGACY_PARAM
            .save(&mut storage, &populated_legacy_param())
            .unwrap();

        do_min_liquidation_value_backfill(&mut storage).unwrap();

        // Operator hand-tunes the floor after the migration.
        let mut tuned = PARAM.load(&storage).unwrap();
        tuned.min_liquidation_value = UsdValue::new_int(42);
        PARAM.save(&mut storage, &tuned).unwrap();

        do_min_liquidation_value_backfill(&mut storage).unwrap();

        let after = PARAM.load(&storage).unwrap();
        assert_eq!(after.min_liquidation_value, UsdValue::new_int(42));
    }

    mod gateway_rolling_window {
        use {
            super::super::{do_gateway_rolling_window_seed, legacy_gateway::OUTBOUND_QUOTAS},
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
}
