mod oracle;
mod perps;

use {
    grug_app::AppResult,
    grug_types::{BlockInfo, Storage},
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // Migrate the oracle's price sources from a single source per denom to a
    // weighted list of sources per denom.
    oracle::do_oracle_upgrades(storage)?;

    Ok(())
}
