mod perps;
mod taxman;

use {
    dango_app::AppResult,
    dango_primitives::{BlockInfo, Storage},
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // Inline the gas-fee logic that previously lived in the taxman contract.
    taxman::do_taxman_removal_upgrade(storage)
}
