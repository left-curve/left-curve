mod perps;

use {
    grug_app::AppResult,
    grug_types::{BlockInfo, Storage},
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    tracing::info!("running perps reduce-only order cleanup migration");

    perps::do_perps_upgrades(storage)?;

    Ok(())
}
