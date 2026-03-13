use {
    dango_types::{
        constants::usdc,
        taxman::{CommissionRebound, ReferralConfig, ShareRatio},
    },
    grug::{Addr, Inner, NumberConst, Storage, Udec128, Uint128, addr, btree_map},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
};

/// Address of the taxman contract.
const TAXMAN: Addr = addr!("da70a9c1417aee00f960fe896add9d571f9c365b");

/// Old Config layout.
mod legacy_taxman {
    use grug::{Denom, Item, Udec128};

    #[grug::derive(Serde, Borsh)]
    pub struct Config {
        pub fee_denom: Denom,
        pub fee_rate: Udec128,
    }

    pub const CONFIG: Item<Config> = Item::new("config");
}

pub fn do_upgrade(storage: Box<dyn Storage>) -> AppResult<()> {
    let mut taxman_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, TAXMAN.inner()]);

    let old_cfg = legacy_taxman::CONFIG.load(&mut taxman_storage)?;

    tracing::info!(fee_denom = %old_cfg.fee_denom, "Loaded legacy taxman config");

    let new_cfg = dango_types::taxman::Config {
        fee_denom: old_cfg.fee_denom,
        fee_rate: old_cfg.fee_rate,
        referral: ReferralConfig {
            max_share_rate: ShareRatio::new(Udec128::new_percent(50))?,
            volume_to_be_referrer: Uint128::new(10_000 * 10_u128.pow(usdc::DECIMAL)), // 10k USDC
            commission_rebound_default: CommissionRebound::new(Udec128::ZERO)?,
            commission_rebound_by_volume: btree_map!(),
        },
    };

    dango_taxman::CONFIG.save(&mut taxman_storage, &new_cfg)?;

    tracing::info!("Migrated taxman config with default referral settings");

    Ok(())
}
