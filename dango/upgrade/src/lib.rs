mod delete_multisig;
mod migrate_users;

use {
    grug::{BlockInfo, Storage},
    grug_app::AppResult,
};

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    // `delete_multisig` runs first — it migrates the ACCOUNTS format.
    // `migrate_users` then reads the migrated ACCOUNTS to build User structs.
    delete_multisig::do_upgrade(storage.clone())?;

    migrate_users::do_upgrade(storage)
}
