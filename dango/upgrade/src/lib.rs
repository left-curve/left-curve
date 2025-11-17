use {
    dango_types::constants::usdc,
    grug::{Addr, BlockInfo, Number, Order, StdError, StdResult, Storage, Udec128_6, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
};

const DEX: Addr = addr!("8dd37b7e12d36bbe1c00ce9f0c341bfe1712e73f");

const SCALING_FACTOR: Udec128_6 = Udec128_6::new(10_u128.pow(usdc::DECIMAL));

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, _block: BlockInfo) -> AppResult<()> {
    tracing::info!("Deleting DEX volume data");

    let mut dex_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, &DEX]);

    for ((address, timestamp), volume) in dango_dex::VOLUMES
        .range(&dex_storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        let new_volume = volume.checked_mul(SCALING_FACTOR).map_err(StdError::from)?;

        dango_dex::VOLUMES.save(&mut dex_storage, (&address, timestamp), &new_volume)?;

        tracing::info!(
            %address,
            timestamp = timestamp.to_rfc3339_string(),
            %volume,
            %new_volume,
            "Migrated volume"
        );
    }

    for ((username, timestamp), volume) in dango_dex::VOLUMES_BY_USER
        .range(&dex_storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?
    {
        let new_volume = volume.checked_mul(SCALING_FACTOR).map_err(StdError::from)?;

        dango_dex::VOLUMES_BY_USER.save(&mut dex_storage, (&username, timestamp), &new_volume)?;

        tracing::info!(
            %username,
            timestamp = timestamp.to_rfc3339_string(),
            %volume,
            %new_volume,
            "Migrated volume by user"
        );
    }

    tracing::info!("Completed deleting DEX volume data");

    Ok(())
}
