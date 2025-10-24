#[cfg(feature = "metrics")]
use grug_types::MetricsIterExt;
use {
    crate::{DbError, DbResult, digest::batch_hash},
    grug_app::{Db, PrunableDb},
    grug_db_disk_types::{BatchBuilder, ColumnFamily, PlainCf, U64Timestamp, VersionedCf, open_db},
    grug_types::{Batch, Empty, Hash256, Op, Order, Record, Storage},
    ouroboros::self_referencing,
    rocksdb::{DBWithThreadMode, MultiThreaded},
    std::{
        collections::BTreeMap,
        ops::Bound,
        path::Path,
        sync::{Arc, RwLock, RwLockReadGuard},
    },
};

const LATEST_VERSION_KEY: &[u8] = b"version";

const OLDEST_VERSION_KEY: &[u8] = b"oldest_version";

const LATEST_BATCH_HASH_KEY: &[u8] = b"hash";

const STORAGE: VersionedCf = ColumnFamily::new("storage");

const METADATA: PlainCf = ColumnFamily::new("metadata");

#[cfg(feature = "metrics")]
const DISK_DB_LITE_LABEL: &str = "grug.db.disk_lite.duration";

pub struct DiskDbLite {
    db: Arc<DBWithThreadMode<MultiThreaded>>,
    pending_data: RwLock<Option<PendingData>>,
    priority_data: Option<Arc<PriorityData>>,
}

pub(crate) struct PendingData {
    version: u64,
    hash: Hash256,
    batch: Batch,
}

struct PriorityData {
    min: Vec<u8>, // inclusive
    max: Vec<u8>, // exclusive
    records: RwLock<BTreeMap<Vec<u8>, Vec<u8>>>,
}

impl DiskDbLite {
    /// Create a new instance of `DiskDbLite` by opening a RocksDB folder on-disk.
    ///
    /// ## Parameters
    ///
    /// - `data_dir`: Path of the RocksDB data directory.
    ///
    /// - `priority_range`: An optional range of keys; if provided, all records
    ///   within this range are loaded into an in-memory B-tree map. Reading or
    ///   iterating records within this range enjoys higher performance, thanks
    ///   to not having to access the disk, at the cost of higher memory usage.
    ///
    /// In Dango, we use `priority_range` for the DEX contract. We observe a
    /// more than 10x performance enhancement compared to not using it (from 43
    /// to 3.9 milliseconds per block). See `examples/auction_benchmark.rs` in
    /// dango-scripts.
    pub fn open<P, B>(data_dir: P, priority_range: Option<&(B, B)>) -> DbResult<Self>
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        let db = open_db(data_dir, [STORAGE.open_opt(), METADATA.open_opt()])?;

        // If `priority_range` is specified, load the data in that range into memory.
        let priority_data = priority_range.map(|(min, max)| {
            #[cfg(feature = "tracing")]
            let mut size = 0;

            let records = STORAGE
                .iter(
                    &db,
                    None,
                    Some(min.as_ref()),
                    Some(max.as_ref()),
                    Order::Ascending,
                )
                .map(|(k, v)| {
                    #[cfg(feature = "tracing")]
                    {
                        size += k.len() + v.len();
                    }

                    (k.to_vec(), v.to_vec())
                })
                .collect::<BTreeMap<_, _>>();

            #[cfg(feature = "tracing")]
            {
                tracing::info!(num_records = records.len(), size, "Loaded priority data");
            }

            Arc::new(PriorityData {
                min: min.as_ref().to_vec(),
                max: max.as_ref().to_vec(),
                records: RwLock::new(records),
            })
        });

        Ok(Self {
            db: Arc::new(db),
            pending_data: RwLock::new(None),
            priority_data,
        })
    }

    pub fn clone_without_priority_data(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            pending_data: RwLock::new(None),
            priority_data: None,
        }
    }
}

impl Db for DiskDbLite {
    type Error = DbError;
    // The lite DB doesn't support Merkle proofs, as it doesn't Merklize the chain state.
    type Proof = Empty;
    // The lite DB doesn't utilize a state commitment storage.
    type StateCommitment = StateStorage;
    type StateStorage = StateStorage;

    fn state_commitment(&self) -> Self::StateCommitment {
        unimplemented!("`DiskDbLite` does not support state commitment");
    }

    fn state_storage_with_comment(
        &self,
        version: Option<u64>,
        comment: &'static str,
    ) -> DbResult<Self::StateStorage> {
        // Read the latest version.
        // If it doesn't exist, this means not even a single batch has been
        // written yet (e.g. during `InitChain`). In this case just use zero.
        let latest_version = self.latest_version().unwrap_or(0);

        // If version is unspecified, use the latest version. Otherwise, make
        // sure it's no newer than the latest version.
        let version = match version {
            Some(version) => {
                if version > latest_version {
                    return Err(DbError::version_too_new(version, latest_version));
                }
                version
            },
            None => latest_version,
        };

        // If the oldest version record exists (meaning, pruning has been
        // performed at least once), and the requested version is older than it,
        // return error.
        if let Some(oldest_version) = self.oldest_version() {
            if version < oldest_version {
                return Err(DbError::version_too_old(version, oldest_version));
            }
        }

        Ok(StateStorage {
            db: self.db.clone(),
            priority_data: self.priority_data.clone(),
            version,
            comment,
        })
    }

    fn latest_version(&self) -> Option<u64> {
        METADATA.read(&self.db, LATEST_VERSION_KEY).map(|bytes| {
            assert_eq!(
                bytes.len(),
                8,
                "latest DB version is of incorrect byte length: {}",
                bytes.len()
            );
            u64::from_le_bytes(bytes.try_into().unwrap())
        })
    }

    fn root_hash(&self, version: Option<u64>) -> DbResult<Option<Hash256>> {
        if let Some(version) = version {
            if version != self.latest_version().unwrap_or(0) {
                return Ok(None);
            }
        }

        Ok(METADATA.read(&self.db, LATEST_BATCH_HASH_KEY).map(|bytes| {
            assert_eq!(
                bytes.len(),
                Hash256::LENGTH,
                "latest DB batch hash is of incorrect byte length: {}",
                bytes.len()
            );
            Hash256::from_inner(bytes.try_into().unwrap())
        }))
    }

    fn prove(&self, _key: &[u8], _version: Option<u64>) -> DbResult<Self::Proof> {
        Err(DbError::proof_unsupported())
    }

    fn flush_but_not_commit(&self, batch: Batch) -> DbResult<(u64, Option<Hash256>)> {
        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        // A pending data can't already exist.
        if self.pending_data.read()?.is_some() {
            return Err(DbError::pending_data_already_set());
        }

        // If DB is empty (latest height doesn't exist), then we use zero as the
        // initial version. This ensures DB version and block height are always
        // the same.
        // TODO: this behavior may need to change once we switch to Malachite.
        let version = self.latest_version().map(|v| v + 1).unwrap_or(0);

        // Since the Lite DB doesn't Merklize the state, we generate a hash based
        // on the changeset.
        let hash = batch_hash(&batch);

        *(self.pending_data.write()?) = Some(PendingData {
            version,
            hash,
            batch,
        });

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LITE_LABEL, "operation" => "flush_but_not_commit")
                .record(duration.elapsed().as_secs_f64());
        }

        Ok((version, Some(hash)))
    }

    fn commit(&self) -> DbResult<()> {
        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        // A pending data must already exists.
        let pending = self
            .pending_data
            .write()?
            .take()
            .ok_or(DbError::pending_data_not_set())?;

        // If priority data exists, apply the change set to it.
        if let Some(data) = &self.priority_data {
            #[cfg(feature = "tracing")]
            {
                tracing::info!("Locking priority data for writing"); // FIXME: change to `debug!` once we figure out the deadlock issue
            }

            let mut records = data.records.write().expect("priority records poisoned");
            for (k, op) in pending.batch.range::<[u8], _>((
                Bound::Included(data.min.as_slice()),
                Bound::Excluded(data.max.as_slice()),
            )) {
                if let Op::Insert(v) = op {
                    records.insert(k.clone(), v.clone());
                } else {
                    records.remove(k);
                }
            }

            #[cfg(feature = "tracing")]
            {
                tracing::info!("Releasing the lock on priority data"); // FIXME: change to `debug!` once we figure out the deadlock issue
            }
        }

        // Now, prepare the write batch that will be written to RocksDB.
        let mut batch_builder =
            BatchBuilder::new(&self.db).with_timestamp(U64Timestamp::from(pending.version));

        // Wriet batch to default CF.
        // let cf = cf_handle(&self.inner.db, CF_NAME_DEFAULT);
        batch_builder.update(STORAGE, |batch| {
            for (k, op) in pending.batch {
                if let Op::Insert(v) = op {
                    batch.put(k, v);
                } else {
                    batch.delete(k);
                }
            }
        });

        // Write version and hash to metadata CF.
        // let cf = cf_handle(&self.inner.db, CF_NAME_METADATA);
        // batch.put_cf(&cf, LATEST_VERSION_KEY, pending.version.to_le_bytes());
        // batch.put_cf(&cf, LATEST_BATCH_HASH_KEY, pending.hash);

        batch_builder.update(METADATA, |batch| {
            batch.put(LATEST_VERSION_KEY, pending.version.to_le_bytes());
            batch.put(LATEST_BATCH_HASH_KEY, pending.hash);
        });

        batch_builder.commit()?;

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LITE_LABEL, "operation" => "commit")
                .record(duration.elapsed().as_secs_f64());
        }

        Ok(())
    }
}

impl PrunableDb for DiskDbLite {
    fn oldest_version(&self) -> Option<u64> {
        METADATA.read(&self.db, OLDEST_VERSION_KEY).map(|bytes| {
            assert_eq!(
                bytes.len(),
                8,
                "oldest DB version is of incorrect byte length: {}",
                bytes.len()
            );
            u64::from_le_bytes(bytes.try_into().unwrap())
        })
    }

    fn prune(&self, up_to_version: u64) -> Result<(), Self::Error> {
        let ts = U64Timestamp::from(up_to_version);

        let cf = STORAGE.cf_handle(&self.db);
        self.db.increase_full_history_ts_low(&cf, ts)?;

        let mut batch = BatchBuilder::new(&self.db);

        batch.update(METADATA, |batch| {
            batch.put(OLDEST_VERSION_KEY, up_to_version.to_le_bytes());
        });

        batch.commit()?;

        Ok(())
    }
}

// ------------------------------- state storage -------------------------------

#[derive(Clone)]
pub struct StateStorage {
    db: Arc<DBWithThreadMode<MultiThreaded>>,
    priority_data: Option<Arc<PriorityData>>,
    version: u64,
    #[cfg_attr(not(feature = "metrics"), allow(dead_code))]
    comment: &'static str,
}

impl StateStorage {
    fn create_iterator<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        // If priority data exists, and the iterator range completely falls
        // within the priority range, then create a priority iterator.
        // Note: `min` is inclusive, while `max` is exclusive.
        if let (Some(data), Some(min), Some(max)) = (&self.priority_data, min, max) {
            if data.min.as_slice() <= min && max < data.max.as_slice() {
                return self.create_priority_iterator(data, min, max, order);
            }
        }

        self.create_non_priority_iterator(min, max, order)
    }

    fn create_priority_iterator<'a>(
        &'a self,
        data: &'a PriorityData,
        min: &[u8],
        max: &[u8],
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let records = data.records.read().expect("priority records poisoned");
        let iter = PriorityIter::new(records, |guard| guard.scan(Some(min), Some(max), order));

        #[cfg(feature = "metrics")]
        let iter = iter.with_metrics(DISK_DB_LITE_LABEL, [
            ("operation", "next_priority"),
            ("comment", self.comment),
        ]);

        Box::new(iter)
    }

    fn create_non_priority_iterator<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let iter = STORAGE.iter(&self.db, Some(self.version), min, max, order);

        #[cfg(feature = "metrics")]
        let iter = iter.with_metrics(DISK_DB_LITE_LABEL, [
            ("operation", "next"),
            ("comment", self.comment),
        ]);

        Box::new(iter)
    }
}

impl Storage for StateStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        // If the key falls in the priority data range, read the value from
        // priority data, without accessing the disk.
        // Note: `min` is inclusive, while `max` is exclusive.
        if let Some(data) = &self.priority_data {
            if data.min.as_slice() <= key && key < data.max.as_slice() {
                return data
                    .records
                    .read()
                    .expect("priority records poisoned")
                    .get(key)
                    .cloned();
            }
        }

        let result = STORAGE.read(&self.db, Some(self.version), key);

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LITE_LABEL, "operation" => "read", "comment" => self.comment)
                .record(duration.elapsed().as_secs_f64());
        }

        result
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        let iter = self.create_iterator(min, max, order);

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LITE_LABEL, "operation" => "scan", "comment" => self.comment)
                .record(duration.elapsed().as_secs_f64());
        }

        iter
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        let iter = self.create_iterator(min, max, order).map(|(k, _)| k);

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LITE_LABEL, "operation" => "scan_keys", "comment" => self.comment)
                .record(duration.elapsed().as_secs_f64());
        }

        Box::new(iter)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        let iter = self.create_iterator(min, max, order).map(|(_, v)| v);

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LITE_LABEL, "operation" => "scan_values", "comment" => self.comment)
                .record(duration.elapsed().as_secs_f64());
        }

        Box::new(iter)
    }

    fn write(&mut self, _key: &[u8], _value: &[u8]) {
        unreachable!("write function called on read-only storage");
    }

    fn remove(&mut self, _key: &[u8]) {
        unreachable!("write function called on read-only storage");
    }

    fn remove_range(&mut self, _min: Option<&[u8]>, _max: Option<&[u8]>) {
        unreachable!("write function called on read-only storage");
    }
}

#[self_referencing]
struct PriorityIter<'a> {
    guard: RwLockReadGuard<'a, BTreeMap<Vec<u8>, Vec<u8>>>,
    #[borrows(guard)]
    #[covariant]
    inner: Box<dyn Iterator<Item = Record> + 'this>,
}

impl<'a> Iterator for PriorityIter<'a> {
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        self.with_inner_mut(|iter| iter.next())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug_types::hash, rocksdb::CompactOptions, temp_rocksdb::TempDataDir};

    // sha256(6 | donald | 1 | 5 | trump | 4 | jake | 1 | 8 | shepherd | 3 | joe | 1 | 5 | biden | 5 | larry | 1 | 8 | engineer)
    // = sha256(0006646f6e616c640100057472756d7000046a616b65010008736865706865726400036a6f65010005626964656e00056c61727279010008656e67696e656572)
    const V0_HASH: Hash256 =
        hash!("be33ce9316ee2af84f037db3a9d6d01bd2e61557ae7859d4d02138b08e6cc9f9");

    // sha256(6 | donald | 1 | 4 | duck | 3 | joe | 0 | 7 | pumpkin | 1 | 3 | cat)
    // = sha256(0006646f6e616c640100046475636b00036a6f6500000770756d706b696e010003636174)
    const V1_HASH: Hash256 =
        hash!("27fc5226bce75bd7750366ee3ddcf35f2d8daafb9f8e14f855f673e1e6fcb021");

    #[test]
    fn disk_db_lite_works() {
        let path = TempDataDir::new("_grug_disk_db_lite_works");
        let db = DiskDbLite::open::<_, Vec<u8>>(&path, None).unwrap();

        // Write a 1st batch.
        {
            let batch = Batch::from([
                (b"donald".to_vec(), Op::Insert(b"trump".to_vec())),
                (b"jake".to_vec(), Op::Insert(b"shepherd".to_vec())),
                (b"joe".to_vec(), Op::Insert(b"biden".to_vec())),
                (b"larry".to_vec(), Op::Insert(b"engineer".to_vec())),
            ]);
            let (version, root_hash) = db.flush_and_commit(batch).unwrap();
            assert_eq!(version, 0);
            assert_eq!(root_hash, Some(V0_HASH));

            for (k, v) in [
                ("donald", Some("trump")),
                ("jake", Some("shepherd")),
                ("joe", Some("biden")),
                ("larry", Some("engineer")),
            ] {
                let found_value = db
                    .state_storage(Some(version))
                    .unwrap()
                    .read(k.as_bytes())
                    .map(|v| String::from_utf8(v).unwrap());
                assert_eq!(found_value.as_deref(), v);
            }
        }

        // Write a 2nd batch.
        {
            let batch = Batch::from([
                (b"donald".to_vec(), Op::Insert(b"duck".to_vec())),
                (b"joe".to_vec(), Op::Delete),
                (b"pumpkin".to_vec(), Op::Insert(b"cat".to_vec())),
            ]);
            let (version, root_hash) = db.flush_and_commit(batch).unwrap();
            assert_eq!(version, 1);
            assert_eq!(root_hash, Some(V1_HASH));

            for (k, v) in [
                ("donald", Some("duck")),
                ("jake", Some("shepherd")),
                ("joe", None),
                ("larry", Some("engineer")),
                ("pumpkin", Some("cat")),
            ] {
                let found_value = db
                    .state_storage(Some(version))
                    .unwrap()
                    .read(k.as_bytes())
                    .map(|v| String::from_utf8(v).unwrap());
                assert_eq!(found_value.as_deref(), v);
            }
        }
    }

    #[test]
    fn prune_works() {
        let path = TempDataDir::new("_grug_disk_db_lite_pruning_works");
        let db = DiskDbLite::open::<_, Vec<u8>>(&path, None).unwrap();

        for batch in [
            // v0
            Batch::from([(b"0".to_vec(), Op::Insert(b"0".to_vec()))]),
            // v1
            Batch::from([(b"1".to_vec(), Op::Insert(b"1".to_vec()))]),
            // v2
            Batch::from([(b"2".to_vec(), Op::Insert(b"2".to_vec()))]),
            // v3
            Batch::from([(b"3".to_vec(), Op::Insert(b"3".to_vec()))]),
            // v4
            Batch::from([(b"4".to_vec(), Op::Insert(b"4".to_vec()))]),
        ] {
            db.flush_and_commit(batch).unwrap();
        }

        let current_version = db.latest_version();

        assert_eq!(current_version, Some(4));

        let storage = db.state_storage(current_version).unwrap();

        assert_eq!(storage.read(b"2"), Some(b"2".to_vec()));
        assert_eq!(storage.read(b"4"), Some(b"4".to_vec()));

        db.prune(3).unwrap();

        let storage = db.state_storage(current_version).unwrap();

        assert_eq!(storage.read(b"2"), Some(b"2".to_vec()));
        assert_eq!(storage.read(b"4"), Some(b"4".to_vec()));

        db.flush_and_commit(Batch::from([(b"2".to_vec(), Op::Insert(b"22".to_vec()))]))
            .unwrap();

        let storage = db.state_storage(Some(5)).unwrap();

        assert_eq!(storage.read(b"2"), Some(b"22".to_vec()));

        db.prune(3).unwrap();

        db.db.compact_range_cf_opt(
            &STORAGE.cf_handle(&db.db),
            None::<&[u8]>,
            None::<&[u8]>,
            &CompactOptions::default(),
        );

        let storage = db.state_storage(Some(4)).unwrap();
        assert_eq!(storage.read(b"2"), Some(b"2".to_vec()));

        let storage = db.state_storage(Some(3)).unwrap();
        assert_eq!(storage.read(b"2"), Some(b"2".to_vec()));

        let cf = STORAGE.cf_handle(&db.db);
        let read_opts = STORAGE.read_options(Some(2));

        // try to read v2, should fail
        db.db.get_cf_opt(&cf, b"2", &read_opts).unwrap_err();
    }
}
