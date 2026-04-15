//! Gateway rate-limit migration.
//!
//! Migrates from the old single-epoch `outbound_quota` accumulator to the new
//! epoch-based per-user rate limiting with a 24-slot sliding window.
//!
//! 1. Initialize `EPOCH` to 0.
//! 2. Snapshot `SUPPLIES` for each rate-limited denom (read from bank storage).
//! 3. Delete the defunct `outbound_quota` map.

use {
    dango_gateway::{EPOCH, RATE_LIMITS, SUPPLIES},
    dango_types::config::AppConfig,
    grug::{BlockInfo, Denom, JsonDeExt, Map, Storage, Uint128},
    grug_app::{APP_CONFIG, AppResult, CONFIG, CONTRACT_NAMESPACE, StorageProvider},
};

/// Old storage key from the previous rate-limit implementation.
const OLD_OUTBOUND_QUOTAS: Map<&Denom, Uint128> = Map::new("outbound_quota");

/// Bank contract's supply map (same key as `dango_bank::SUPPLIES`).
const BANK_SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let config = CONFIG.load(&*storage)?;
    let app_config: AppConfig = APP_CONFIG.load(&*storage)?.deserialize_json()?;

    let bank_addr = config.bank;
    let gateway_addr = app_config.addresses.gateway;

    let bank_storage = StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, &bank_addr]);
    let mut gw_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &gateway_addr]);

    // 1. Initialize the epoch counter.
    EPOCH.save(&mut gw_storage, &0)?;

    tracing::info!("Initialized EPOCH to 0");

    // 2. Snapshot supply for each rate-limited denom by reading from the bank.
    let rate_limits = RATE_LIMITS.load(&gw_storage)?;

    for denom in rate_limits.keys() {
        let supply = BANK_SUPPLIES
            .may_load(&bank_storage, denom)?
            .unwrap_or_default();

        SUPPLIES.save(&mut gw_storage, denom, &supply)?;

        tracing::info!(denom = %denom, supply = %supply, "Snapshotted supply");
    }

    // 3. Delete the old `outbound_quota` map.
    OLD_OUTBOUND_QUOTAS.clear(&mut gw_storage, None, None);

    tracing::info!("Cleared old outbound_quota map");

    Ok(())
}
