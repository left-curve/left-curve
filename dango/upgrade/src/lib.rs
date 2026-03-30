use {
    dango_types::perps::{Param, RateSchedule},
    grug::{Addr, Inner, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
};

/// Address of the perps contract.
const PERPS: Addr = addr!("d04b99adca5d3d31a1e7bc72fd606202f1e2fc69");

/// Old `Param` struct with the `referral: ReferralParam` field.
mod legacy {
    use {
        dango_types::{Dimensionless, UsdValue},
        grug::{Duration, Item},
        std::collections::BTreeMap,
    };

    #[grug::derive(Serde, Borsh)]
    #[derive(Default)]
    pub struct ReferralParam {
        pub active: bool,
        pub volume_to_be_referrer: UsdValue,
        pub commission_rate_default: Dimensionless,
        pub commission_rates_by_volume: BTreeMap<UsdValue, Dimensionless>,
    }

    #[grug::derive(Serde, Borsh)]
    pub struct Param {
        pub max_unlocks: usize,
        pub max_open_orders: usize,
        pub max_conditional_orders: usize,
        pub base_maker_fee_rate: Dimensionless,
        pub base_taker_fee_rate: Dimensionless,
        pub tiered_maker_fee_rate: BTreeMap<UsdValue, Dimensionless>,
        pub tiered_taker_fee_rate: BTreeMap<UsdValue, Dimensionless>,
        pub protocol_fee_rate: Dimensionless,
        pub liquidation_fee_rate: Dimensionless,
        pub funding_period: Duration,
        pub vault_total_weight: Dimensionless,
        pub vault_cooldown_period: Duration,
        pub referral: ReferralParam,
    }

    pub const PARAM: Item<Param> = Item::new("param");
}

pub fn do_upgrade<VM>(
    storage: Box<dyn Storage>,
    _vm: VM,
    _block: grug::BlockInfo,
) -> AppResult<()> {
    let mut perps_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, PERPS.inner()]);

    // Load the old Param (with nested `referral: ReferralParam`).
    let old = legacy::PARAM.load(&perps_storage)?;

    // Save the new Param with RateSchedule fields and flattened referral.
    let new = Param {
        max_unlocks: old.max_unlocks,
        max_open_orders: old.max_open_orders,
        max_conditional_orders: old.max_conditional_orders,
        maker_fee_rates: RateSchedule {
            base: old.base_maker_fee_rate,
            tiers: old.tiered_maker_fee_rate,
        },
        taker_fee_rates: RateSchedule {
            base: old.base_taker_fee_rate,
            tiers: old.tiered_taker_fee_rate,
        },
        protocol_fee_rate: old.protocol_fee_rate,
        liquidation_fee_rate: old.liquidation_fee_rate,
        funding_period: old.funding_period,
        vault_total_weight: old.vault_total_weight,
        vault_cooldown_period: old.vault_cooldown_period,
        referral_active: old.referral.active,
        min_referrer_volume: old.referral.volume_to_be_referrer,
        referrer_commission_rates: RateSchedule {
            base: old.referral.commission_rate_default,
            tiers: old.referral.commission_rates_by_volume,
        },
    };

    dango_perps::PARAM.save(&mut perps_storage, &new)?;

    tracing::info!("Migrated perps Param: RateSchedule + flattened referral");

    Ok(())
}
