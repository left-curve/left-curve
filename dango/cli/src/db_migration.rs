use {
    grug_app::SimpleCommitment,
    grug_types::{BlockInfo, BorshDeExt, BorshSerExt},
    rocksdb::{DB, DBWithThreadMode, IteratorMode, MultiThreaded, Options, WriteBatch},
    std::path::PathBuf,
};

pub fn migrate_db(path: PathBuf) {
    tracing::info!("Migrating database");

    // ----------------------- Read data from the old DB -----------------------

    // Old DB:
    // - metadata (latest version, latest apphash) in "metadata" -- not timestamped
    // - state storage in "default" -- not timestamped
    let db = DB::open_cf(&Options::default(), &path, ["default", "metadata"]).unwrap();

    // Read the version.
    let latest_version_raw = db
        .get_cf(db.cf_handle("metadata").unwrap(), "version")
        .unwrap()
        .unwrap()
        .try_into()
        .unwrap();
    let latest_version = u64::from_le_bytes(latest_version_raw);

    tracing::info!(latest_version, "Loaded DB metadata");

    // Read the last finalized block
    let last_finalized_block = db
        .get_cf(db.cf_handle("default").unwrap(), b"last_finalized_block")
        .unwrap()
        .unwrap()
        .deserialize_borsh::<BlockInfo>()
        .unwrap();

    tracing::info!(
        height = last_finalized_block.height,
        hash = last_finalized_block.hash.to_string(),
        "Loaded last finalized block"
    );

    // Load the entire state storage.
    let records = db
        .iterator_cf(db.cf_handle("default").unwrap(), IteratorMode::Start)
        .map(|res| {
            let (k, v) = res.unwrap();
            (k.to_vec(), v.to_vec())
        })
        .collect::<Vec<_>>();

    tracing::info!(num_records = records.len(), "Loaded state storage");

    // Delete the database entirely.
    drop(db);
    DB::destroy(&Options::default(), &path).unwrap();
    assert!(
        !path.exists(),
        "expect data dir to be deleted, but it still exists"
    );

    tracing::info!(path = path.to_str().unwrap(), "Deleted old database");

    // ----------------------- Write data to the new DB ------------------------

    // New DB:
    // - metadata (latest version, oldest version) in "default" -- not timestamped
    // - state commitment in "state_commitment" -- not timestamped
    // - state storage in "state_storage" -- *timestamped*
    let db = DBWithThreadMode::<MultiThreaded>::open_cf_with_opts(
        &grug_db_disk::new_db_options(),
        path,
        [
            (grug_db_disk::CF_NAME_DEFAULT, Options::default()),
            (grug_db_disk::CF_NAME_STATE_COMMITMENT, Options::default()),
            (
                grug_db_disk::CF_NAME_STATE_STORAGE,
                grug_db_disk::new_cf_options_with_ts(), // IMPORTANT: with timestamp
            ),
        ],
    )
    .unwrap();

    let mut batch = WriteBatch::new();

    // Write the version. The latest version of the old DB is simultaneously
    // the oldest and latest version in the new DB.
    let cf = grug_db_disk::cf_default(&db);
    batch.put_cf(&cf, grug_db_disk::LATEST_VERSION_KEY, latest_version_raw);
    batch.put_cf(&cf, grug_db_disk::OLDEST_VERSION_KEY, latest_version_raw);

    // Write state commitment.
    let cf = grug_db_disk::cf_state_commitment(&db);
    batch.put_cf(
        &cf,
        SimpleCommitment::ROOT_HASHES
            .path(last_finalized_block.height)
            .storage_key(),
        last_finalized_block.hash.to_borsh_vec().unwrap(),
    );

    // Write state storage (with timestamp).
    let cf = grug_db_disk::cf_state_storage(&db);
    let ts = grug_db_disk::U64Timestamp::from(latest_version);
    for (key, value) in records {
        batch.put_cf_with_ts(&cf, key, ts, value);
    }

    db.write(batch).unwrap();

    tracing::info!("Database migration completed");
}
