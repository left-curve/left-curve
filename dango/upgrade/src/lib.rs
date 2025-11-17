use {
    grug::{Addr, BlockInfo, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
};

/// Address of the DEX contract.
const DEX: Addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    tracing::info!("Deleting DEX volume data");

    // Get the storage of the DEX contract.
    let mut dex_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &DEX]);

    dango_dex::VOLUMES.clear(&mut dex_storage, None, None);
    dango_dex::VOLUMES_BY_USER.clear(&mut dex_storage, None, None);

    tracing::info!("Completed deleting DEX volume data");

    Ok(())
}
