mod gateway;
mod perps;

use {
    grug::{BlockInfo, Storage},
    grug_app::AppResult,
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    gateway::do_gateway_upgrades(storage)
}
