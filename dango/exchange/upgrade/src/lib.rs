mod perps;

use {
    dango_app::AppResult,
    dango_primitives::{BlockInfo, Storage},
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    perps::do_perps_upgrades(storage)?;

    Ok(())
}
