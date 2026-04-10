use {
    dango_types::{
        Dimensionless, UsdValue,
        perps::{self, RateSchedule},
    },
    grug::{Addr, BlockInfo, Duration, StdResult, Storage, addr},
    grug_app::{AppResult, CHAIN_ID, CONTRACT_NAMESPACE, StorageProvider},
};

const MAINNET_CHAIN_ID: &str = "dango-1";
const MAINNET_PERPS_ADDRESS: Addr = addr!("90bc84df68d1aa59a857e04ed529e9a26edbea4f");

const TESTNET_CHAIN_ID: &str = "dango-testnet-1";
const TESTNET_PERPS_ADDRESS: Addr = addr!("f6344c5e2792e8f9202c58a2d88fbbde4cd3142f");

/// Legacy types matching the pre-upgrade Borsh layout.
mod legacy {
    use super::*;

    pub const PARAM: grug::Item<Param> = grug::Item::new("param");

    /// The Param struct before this upgrade, which does not contain the
    /// `vault_deposit_cap` field.
    #[derive(borsh::BorshDeserialize, borsh::BorshSerialize)]
    pub struct Param {
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
    }
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let chain_id = CHAIN_ID.load(&storage)?;

    let perps_address = match chain_id.as_str() {
        MAINNET_CHAIN_ID => MAINNET_PERPS_ADDRESS,
        TESTNET_CHAIN_ID => TESTNET_PERPS_ADDRESS,
        _ => panic!("unknown chain id: {chain_id}"),
    };

    let mut storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &perps_address]);

    Ok(_do_upgrade(&mut storage)?)
}

fn _do_upgrade(storage: &mut dyn Storage) -> StdResult<()> {
    let old_param = legacy::PARAM.load(storage)?;

    let new_param = perps::Param {
        max_unlocks: old_param.max_unlocks,
        max_open_orders: old_param.max_open_orders,
        maker_fee_rates: old_param.maker_fee_rates,
        taker_fee_rates: old_param.taker_fee_rates,
        protocol_fee_rate: old_param.protocol_fee_rate,
        liquidation_fee_rate: old_param.liquidation_fee_rate,
        liquidation_buffer_ratio: old_param.liquidation_buffer_ratio,
        funding_period: old_param.funding_period,
        vault_total_weight: old_param.vault_total_weight,
        vault_cooldown_period: old_param.vault_cooldown_period,
        referral_active: old_param.referral_active,
        min_referrer_volume: old_param.min_referrer_volume,
        referrer_commission_rates: old_param.referrer_commission_rates,
        vault_deposit_cap: Some(UsdValue::new_int(500_000)),
    };

    dango_perps::state::PARAM.save(storage, &new_param)?;

    tracing::info!("Migrated Param (added vault_deposit_cap = $500,000)");

    Ok(())
}
