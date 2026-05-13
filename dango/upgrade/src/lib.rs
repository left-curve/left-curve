use {
    dango_order_book::{Dimensionless, UsdValue},
    dango_perps::state::PARAM,
    dango_types::perps::{Param, RateSchedule},
    grug::{Addr, BlockInfo, Duration, Item, StdResult, Storage, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Floor backfilled into `Param.min_liquidation_value` on upgrade. Mirrors
/// the genesis value so post-upgrade chains match a fresh chain.
const MIGRATED_MIN_LIQUIDATION_VALUE: UsdValue = UsdValue::new_int(10);

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
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

/// Frozen historical schemas, kept around so migrations can decode pre-
/// upgrade storage bytes. Anything in here represents on-chain layout at
/// some past point in time — do not re-order, rename, or change field
/// types of these structs.
mod legacy {
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

    let legacy = legacy::LEGACY_PARAM.load(storage)?;

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
            legacy::{LEGACY_PARAM, LegacyParam},
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
}
