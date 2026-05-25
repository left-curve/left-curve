mod app_config;
mod gateway;
mod oracle;
mod perps;
mod taxman;

use {
    grug_app::AppResult,
    grug_types::{BlockInfo, Storage},
};

pub fn do_upgrade<VM>(mut storage: Box<dyn Storage>, _vm: VM, block: BlockInfo) -> AppResult<()> {
    app_config::do_app_config_upgrade(&mut *storage)?;
    gateway::do_gateway_upgrades(storage.clone())?;
    oracle::do_oracle_upgrades(storage.clone())?;
    perps::do_perps_upgrades(storage.clone(), block)?;
    taxman::do_taxman_upgrades(storage)
}
