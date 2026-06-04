mod oracle;
mod perps;
mod taxman;

use {
    grug_app::AppResult,
    grug_types::{BlockInfo, Storage},
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // Sweep the taxman's accumulated protocol fees to the chain owner. Cloning
    // the storage handle is safe: it is a `Shared` (`Arc`-backed) buffer, so all
    // migrations below write to the same backing store.
    taxman::sweep_fees_to_owner(storage.clone())?;

    // Migrate the oracle's price sources to the new `PriceConfig` shape.
    oracle::do_oracle_upgrades(storage)?;

    Ok(())
}
