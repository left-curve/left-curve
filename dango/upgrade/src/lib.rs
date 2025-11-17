use {
    dango_types::constants::{btc, dango, eth, sol, usdc},
    grug::{Addr, BlockInfo, Denom, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    std::sync::LazyLock,
};

/// Address of the DEX contract.
const DEX: Addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

/// The pairs to be migrated.
static PAIRS: [(&LazyLock<Denom>, &LazyLock<Denom>); 4] = [
    (&btc::DENOM, &usdc::DENOM),
    (&eth::DENOM, &usdc::DENOM),
    (&sol::DENOM, &usdc::DENOM),
    (&dango::DENOM, &usdc::DENOM),
];

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    tracing::info!("Deleting DEX volume data");

    // Get the storage of the DEX contract.
    let mut dex_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &DEX]);

    for (base_denom, quote_denom) in PAIRS {
        tracing::info!(
            base_denom = base_denom.to_string(),
            quote_denom = quote_denom.to_string(),
            "Deleting DEX volume data of pair"
        );

        dango_dex::VOLUMES.clear(&mut dex_storage, None, None);
        dango_dex::VOLUMES_BY_USER.clear(&mut dex_storage, None, None);
    }

    tracing::info!("Completed deleting DEX volume data");

    Ok(())
}
