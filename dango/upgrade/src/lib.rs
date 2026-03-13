mod delete_multisig;
mod migrate_taxman_config;

use {
    grug::{BlockInfo, Storage},
    grug_app::AppResult,
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    delete_multisig::do_upgrade(storage.clone())?;
    migrate_taxman_config::do_upgrade(storage)?;

    Ok(())
}
