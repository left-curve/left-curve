mod oracle;
mod perps;

use {
    grug_app::AppResult,
    grug_types::{BlockInfo, Storage},
};

pub fn do_upgrade<VM>(_storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // Nothing to do in the current version.

    // oracle::do_oracle_upgrades(storage.clone())?;
    // perps::do_perps_upgrades(storage)?;

    Ok(())
}
