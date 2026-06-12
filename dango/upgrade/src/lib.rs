mod dex_clean_up;
mod perps;

use {
    grug_app::AppResult,
    grug_types::{BlockInfo, Storage},
};

pub use dex_clean_up::clean_up_dex;

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    clean_up_dex(storage)
}
