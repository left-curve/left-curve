use std::path::Path;

pub fn migrate_db(path: &Path) -> anyhow::Result<()> {
    // If the DB deson't exist, or if it exists but is empty, do nothing.
    // If it exists but isn't a directory, error.
    if !path.exists() || path.read_dir()?.next().is_none() {
        tracing::info!(
            ?path,
            "DB path doesn't exist or is empty. Skipping DB migration"
        );

        return Ok(());
    }

    tracing::info!(?path, "Migrating database");

    // ----------------------- Read data from the old DB -----------------------

    // Import traits from the old DB.
    use {grug_app_old::Db as _, grug_types_old::Storage as _};

    // Open the old DB.
    let db_old = grug_db_disk_old::DiskDb::<grug_app_old::SimpleCommitment>::open(path)?;

    tracing::info!(?path, "Opened old DB");

    // Read metadata.
    let latest_version = db_old.latest_version();

    tracing::info!(?latest_version, "Read metadata from old DB");

    // Read data from state storage.
    let state_storage = db_old
        .state_storage(latest_version)?
        .scan(None, None, grug_types_old::Order::Ascending)
        .collect::<Vec<_>>();

    tracing::info!(
        num_records = state_storage.len(),
        "Read records from old state storage"
    );

    // Read data from state commitment.
    let state_commitment = db_old
        .state_commitment()
        .scan(None, None, grug_types_old::Order::Ascending)
        .collect::<Vec<_>>();

    tracing::info!(
        num_records = state_commitment.len(),
        "Read records from old state commitment"
    );

    // Drop and delete the old DB.
    drop(db_old);
    std::fs::remove_dir_all(path)?;

    tracing::info!(?path, "Deleted old DB");

    // ------------------------ Write data to the new DB -----------------------

    // Open new DB.
    let db = rocksdb::DB::open_cf(&grug_db_disk::new_db_options(), path, [
        grug_db_disk::CF_NAME_DEFAULT,
        grug_db_disk::CF_NAME_STATE_COMMITMENT,
        grug_db_disk::CF_NAME_STATE_STORAGE,
    ])?;

    tracing::info!(?path, "Opened new DB");

    let mut batch = rocksdb::WriteBatch::new();

    // Write metadata.
    if let Some(latest_version) = latest_version {
        batch.put_cf(
            grug_db_disk::cf_default(&db),
            grug_db_disk::LATEST_VERSION_KEY,
            latest_version.to_le_bytes(),
        );
    }

    // Write state storage.
    let cf = grug_db_disk::cf_state_storage(&db);
    for (k, v) in state_storage {
        batch.put_cf(cf, k, v);
    }

    // Write state commitment.
    let cf = grug_db_disk::cf_state_commitment(&db);
    for (k, v) in state_commitment {
        batch.put_cf(cf, k, v);
    }

    let num_records = batch.len();
    let size_in_bytes = batch.size_in_bytes();

    db.write(batch)?;

    tracing::info!(num_records, size_in_bytes, ?path, "Written data to new DB");

    Ok(())
}
