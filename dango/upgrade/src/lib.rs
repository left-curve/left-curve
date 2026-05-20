mod app_config;
mod gateway;
mod perps;
mod taxman;

use {
    grug::{BlockInfo, Storage},
    grug_app::AppResult,
};

pub fn do_upgrade<VM>(mut storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    app_config::do_app_config_upgrade(&mut *storage)?;
    gateway::do_gateway_upgrades(storage.clone())?;
    taxman::do_taxman_upgrades(storage.clone())
}
