use {
    dango_account_factory::{ACCOUNTS, MAIN_ACCOUNT},
    dango_types::account_factory::AccountParams,
    grug::{Addr, BlockInfo, Inner, StdResult, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::BTreeMap,
};

const ACCOUNT_FACTORY: Addr = addr!("18d28bafcdf9d4574f920ea004dea2d13ec16f6b");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    let mut account_factory_storage =
        StorageProvider::new(storage, &[CONTRACT_NAMESPACE, ACCOUNT_FACTORY.inner()]);

    // Load all the accounts from the account factory storage.
    let accounts = ACCOUNTS
        .range(&account_factory_storage, None, None, grug::Order::Ascending)
        .collect::<StdResult<BTreeMap<_, _>>>()?;

    for (addr, account) in accounts {
        let AccountParams::Single(params) = account.params else {
            // Only single accounts can be main account.
            continue;
        };

        // Save the main account mapping.
        MAIN_ACCOUNT.save(&mut account_factory_storage, params.owner, &addr)?;
    }

    Ok(())
}
