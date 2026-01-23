use {
    dango_taxman::VOLUME_TIME_GRANULARITY,
    grug::{Addr, BlockInfo, Inner, Order, StdResult, Storage, addr},
    grug_app::{AppResult, CONTRACT_NAMESPACE, StorageProvider},
    std::collections::BTreeMap,
};

/// Address of the DEX contract.
const DEX: Addr = addr!("da32476efe31e535207f0ad690d337a4ebf54a22");

/// Address of the taxman contract.
const TAXMAN: Addr = addr!("da70a9c1417aee00f960fe896add9d571f9c365b");

/// Storage layout of the DEX contract prior to this PR.
mod legacy_dex {
    use {
        dango_types::account_factory::UserIndex,
        grug::{Addr, Map, Timestamp, Udec128_6},
    };

    pub const VOLUMES: Map<(&Addr, Timestamp), Udec128_6> = Map::new("volume");

    pub const VOLUMES_BY_USER: Map<(UserIndex, Timestamp), Udec128_6> = Map::new("volume_by_user");
}

pub fn do_upgrade<VM>(storage: Box<dyn Storage>, _vm: VM, block: BlockInfo) -> AppResult<()> {
    // Create storage object for the DEX contract.
    let mut dex_storage = StorageProvider::new(storage.clone(), &[CONTRACT_NAMESPACE, DEX.inner()]);

    // Create storage object for the taxman contract.
    let mut taxman_storage = StorageProvider::new(storage, &[CONTRACT_NAMESPACE, TAXMAN.inner()]);

    // Load all records in DEX contract `VOLUMES_BY_USER`.
    let old_volumes = legacy_dex::VOLUMES_BY_USER
        .range(&dex_storage, None, None, Order::Ascending)
        .collect::<StdResult<BTreeMap<_, _>>>()?;

    tracing::info!(
        num_records = old_volumes.len(),
        "Loaded records from DEX contract VOLUMES_BY_USER"
    );

    // Delete all records in DEX contract `VOLUMES`..
    legacy_dex::VOLUMES.clear(&mut dex_storage, None, None);

    tracing::info!("Deleted records in DEX contract VOLUMES");

    // Delete all records in DEX contract `VOLUMES_BY_USER`.
    legacy_dex::VOLUMES_BY_USER.clear(&mut dex_storage, None, None);

    tracing::info!("Deleted records in DEX contract VOLUMES_BY_USER");

    // Convert the volumes records to the new format expected by taxman.
    // This involves too changes:
    // 1. For each user, we only need to keep the most recent record.
    //    We will only start to utilize volume data by late Q1, so older data aren't necessary.
    //    To achieve this, we utilize the fact BTreeMap is sorted ascendingly.
    //    This means we can simply iterate through the record descendingly, and
    //    for each user, only take the first record (the one with the biggest timestamp).
    // 2. The one most recent record we want to keep, round it down to the nearest day.
    let new_volumes = old_volumes
        .into_iter()
        .rev() // reverse so we visit the most recent record first
        .fold(BTreeMap::new(), |mut acc, ((user_index, _timestamp), volume)| {
            acc.entry(user_index).or_insert(volume);
            acc
        });

    tracing::info!(
        num_records = new_volumes.len(),
        "Converted volume data to the new format"
    );

    // Find the current timestamp and round it down to the nearest day.
    let timestamp = block.timestamp - block.timestamp % VOLUME_TIME_GRANULARITY;

    // Save the records to taxman contract's storage.
    for (user_index, volume) in new_volumes {
        dango_taxman::VOLUMES_BY_USER.save(
            &mut taxman_storage,
            (user_index, timestamp),
            &volume,
        )?;
    }

    tracing::info!("Migration completed");

    Ok(())
}
