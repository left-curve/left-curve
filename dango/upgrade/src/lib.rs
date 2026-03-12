mod delete_multisig;

use {
    grug::{BlockInfo, Storage},
    grug_app::AppResult,
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    delete_multisig::do_upgrade(storage)?;

    Ok(())
}
