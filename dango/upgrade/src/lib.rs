use {
    dango_types::perps::{Param, ReferralParam},
    grug::{Addr, Inner, Item, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
};

/// Address of the perps contract.
const PERPS: Addr = addr!("d04b99adca5d3d31a1e7bc72fd606202f1e2fc69");

/// Old `Param` struct without the `referral` field.
mod legacy {
    use {
        dango_types::{Dimensionless, UsdValue},
        grug::{Duration, Item},
        std::collections::BTreeMap,
    };

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
    }

    pub const PARAM: Item<Param> = Item::new("param");
}

/// New `PARAM` item using the current `Param` type (includes `referral`).
const PARAM: Item<Param> = Item::new("param");

pub fn do_upgrade<VM>(
    storage: Box<dyn Storage>,
    _vm: VM,
    _block: grug::BlockInfo,
) -> AppResult<()> {
    let mut perps_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, PERPS.inner()]);
    let perps_storage = &mut perps_storage;

    // Load the old Param (without `referral` field).
    let old = legacy::PARAM.load(perps_storage)?;

    // Save the new Param with default ReferralParam.
    let new = Param {
        max_unlocks: old.max_unlocks,
        max_open_orders: old.max_open_orders,
        max_conditional_orders: old.max_conditional_orders,
        base_maker_fee_rate: old.base_maker_fee_rate,
        base_taker_fee_rate: old.base_taker_fee_rate,
        tiered_maker_fee_rate: old.tiered_maker_fee_rate,
        tiered_taker_fee_rate: old.tiered_taker_fee_rate,
        protocol_fee_rate: old.protocol_fee_rate,
        liquidation_fee_rate: old.liquidation_fee_rate,
        funding_period: old.funding_period,
        vault_total_weight: old.vault_total_weight,
        vault_cooldown_period: old.vault_cooldown_period,
        referral: ReferralParam::default(),
    };

    PARAM.save(perps_storage, &new)?;

    tracing::info!("Migrated perps Param: added default ReferralParam");

    Ok(())
}
