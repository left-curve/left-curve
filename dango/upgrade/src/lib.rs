use {
    dango_types::{
        Dimensionless, UsdValue,
        perps::{self, RateSchedule},
    },
    grug::{BlockInfo, Duration, StdResult, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, CONTRACTS, StorageProvider},
};

/// Address of the perps contract.
const PERPS_ADDRESS: grug::Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Legacy types matching the pre-upgrade Borsh layout.
mod legacy {
    use super::*;

    pub const PARAM: grug::Item<Param> = grug::Item::new("param");

    /// The Param struct before this upgrade, which does not contain the
    /// `liquidation_buffer_ratio` field.
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct Param {
        pub max_unlocks: usize,
        pub max_open_orders: usize,
        pub maker_fee_rates: RateSchedule,
        pub taker_fee_rates: RateSchedule,
        pub protocol_fee_rate: Dimensionless,
        pub liquidation_fee_rate: Dimensionless,
        pub funding_period: Duration,
        pub vault_total_weight: Dimensionless,
        pub vault_cooldown_period: Duration,
        pub referral_active: bool,
        pub min_referrer_volume: UsdValue,
        pub referrer_commission_rates: RateSchedule,
    }
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    if !CONTRACTS.has(&storage, PERPS_ADDRESS) {
        tracing::info!("Perps contract not found. Nothing to do");
        return Ok(());
    }

    let mut perps_storage =
        StorageProvider::new(storage, &[CONTRACT_NAMESPACE, PERPS_ADDRESS.as_ref()]);

    Ok(_do_upgrade(&mut perps_storage)?)
}

fn _do_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    // Migrate Param: load the old layout (no liquidation_buffer_ratio),
    // convert to new layout with the field defaulting to zero.
    let old_param = legacy::PARAM.load(storage)?;

    let new_param = perps::Param {
        max_unlocks: old_param.max_unlocks,
        max_open_orders: old_param.max_open_orders,
        maker_fee_rates: old_param.maker_fee_rates,
        taker_fee_rates: old_param.taker_fee_rates,
        protocol_fee_rate: old_param.protocol_fee_rate,
        liquidation_fee_rate: old_param.liquidation_fee_rate,
        liquidation_buffer_ratio: Dimensionless::new_percent(5), // 5%
        funding_period: old_param.funding_period,
        vault_total_weight: old_param.vault_total_weight,
        vault_cooldown_period: old_param.vault_cooldown_period,
        referral_active: old_param.referral_active,
        min_referrer_volume: old_param.min_referrer_volume,
        referrer_commission_rates: old_param.referrer_commission_rates,
    };

    dango_perps::state::PARAM.save(storage, &new_param)?;

    tracing::info!("Migrated Param (added liquidation_buffer_ratio)");

    Ok(())
}
