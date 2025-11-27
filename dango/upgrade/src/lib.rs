use {
    grug::{BlockInfo, Storage},
    grug_app::AppResult,
};

pub fn do_upgrade<VM>(_storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    Ok(())
}
