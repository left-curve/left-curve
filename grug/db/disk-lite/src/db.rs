use {
    crate::{DbError, DbResult, batch_hash},
    grug_app::{ConsensusStorage, Db},
    grug_types::{Batch, Empty, Hash256, Op, Order, Storage},
    rocksdb::{
        BoundColumnFamily, DBWithThreadMode, IteratorMode, MultiThreaded, Options, ReadOptions,
        WriteBatch,
    },
    std::{
        path::Path,
        sync::{Arc, RwLock},
    },
};

const CF_NAME_STORAGE: &str = "storage";

const CF_NAME_CONSENSUS: &str = "consensus";

const CF_NAME_METADATA: &str = "metadata";

const LATEST_VERSION_KEY: &str = "version";

const LATEST_BATCH_HASH_KEY: &str = "hash";

pub struct DiskDbLite {
    inner: Arc<DiskDbLiteInner>,
}

struct DiskDbLiteInner {
    db: DBWithThreadMode<MultiThreaded>,
    storage_pending_data: RwLock<Option<PendingData>>,
    consensus_pending_data: RwLock<Option<Batch>>,
}

pub(crate) struct PendingData {
    version: u64,
    hash: Hash256,
    batch: Batch,
}

impl DiskDbLite {
    pub fn open<P>(data_dir: P) -> DbResult<Self>
    where
        P: AsRef<Path>,
    {
        let db = DBWithThreadMode::open_cf_with_opts(&new_db_options(), data_dir, [
            (CF_NAME_STORAGE, Options::default()),
            (CF_NAME_CONSENSUS, Options::default()),
            (CF_NAME_METADATA, Options::default()),
        ])?;

        Ok(Self {
            inner: Arc::new(DiskDbLiteInner {
                db,
                storage_pending_data: RwLock::new(None),
                consensus_pending_data: RwLock::new(None),
            }),
        })
    }
}

impl Clone for DiskDbLite {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Db for DiskDbLite {
    type Error = DbError;
    // The lite DB doesn't support Merkle proofs, as it doesn't Merklize the chain state.
    type Proof = Empty;
    // The lite DB doesn't utilize a state commitment storage.
    type StateCommitment = StateStorage;
    type StateConsensus = StateConsensus;
    type StateStorage = StateStorage;

    fn state_commitment(&self) -> Self::StateCommitment {
        unimplemented!("`DiskDbLite` does not support state commitment");
    }

    fn state_storage(&self, version: Option<u64>) -> DbResult<Self::StateStorage> {
        // If a version is specified, it must equal the latest version.
        if let Some(requested) = version {
            let db_version = self.latest_version().unwrap_or(0);
            if requested != db_version {
                return Err(DbError::IncorrectVersion {
                    db_version,
                    requested,
                });
            }
        }

        Ok(StateStorage {
            inner: Arc::clone(&self.inner),
            cf_name: CF_NAME_STORAGE,
        })
    }

    fn state_consensus(&self) -> Self::StateConsensus {
        StateConsensus {
            inner: Arc::clone(&self.inner),
            cf_name: CF_NAME_CONSENSUS,
        }
    }

    fn latest_version(&self) -> Option<u64> {
        self.inner
            .db
            .get_cf(
                &cf_handle(&self.inner.db, CF_NAME_METADATA),
                LATEST_VERSION_KEY,
            )
            .unwrap_or_else(|err| {
                panic!("failed to read latest DB version: {err}");
            })
            .map(|bytes| {
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

        Ok(self
            .inner
            .db
            .get_cf(
                &cf_handle(&self.inner.db, CF_NAME_METADATA),
                LATEST_BATCH_HASH_KEY,
            )?
            .map(|bytes| {
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
        Err(DbError::ProofUnsupported)
    }

    fn flush_storage_but_not_commit(&self, batch: Batch) -> DbResult<(u64, Option<Hash256>)> {
        // A pending data can't already exist.
        if self.inner.storage_pending_data.read()?.is_some() {
            return Err(DbError::PendingDataAlreadySet);
        }

        // If DB is empty (latest height doesn't exist), then we use zero as the
        // initial version. This ensures DB version and block height are always
        // the same.
        // TODO: this behavior may need to change once we switch to Malachite.
        let version = self.latest_version().map(|v| v + 1).unwrap_or(0);

        // Since the Lite DB doesn't Merklize the state, we generate a hash based
        // on the changeset.
        let hash = batch_hash(&batch);

        *(self.inner.storage_pending_data.write()?) = Some(PendingData {
            version,
            hash,
            batch,
        });

        Ok((version, Some(hash)))
    }

    fn flush_consensus_but_not_commit(&self, batch: Batch) -> DbResult<()> {
        *(self.inner.consensus_pending_data.write()?) = Some(batch);
        Ok(())
    }

    fn commit(&self) -> DbResult<()> {
        // A pending data must already exists.
        let storage_pending = self
            .inner
            .storage_pending_data
            .write()?
            .take()
            .ok_or(DbError::PendingDataNotSet)?;

        let consensus_pending = self
            .inner
            .consensus_pending_data
            .write()?
            .take()
            .unwrap_or_default();

        let mut batch = WriteBatch::default();

        // Wriet batch to default CF.
        let cf = cf_handle(&self.inner.db, CF_NAME_STORAGE);
        for (k, op) in storage_pending.batch {
            if let Op::Insert(v) = op {
                batch.put_cf(&cf, k, v);
            } else {
                batch.delete_cf(&cf, k);
            }
        }

        // Write consensus batch to consensus CF.
        let cf = cf_handle(&self.inner.db, CF_NAME_CONSENSUS);
        for (k, op) in consensus_pending {
            if let Op::Insert(v) = op {
                batch.put_cf(&cf, k, v);
            } else {
                batch.delete_cf(&cf, k);
            }
        }

        // Write version and hash to metadata CF.
        let cf = cf_handle(&self.inner.db, CF_NAME_METADATA);
        batch.put_cf(
            &cf,
            LATEST_VERSION_KEY,
            storage_pending.version.to_le_bytes(),
        );
        batch.put_cf(&cf, LATEST_BATCH_HASH_KEY, storage_pending.hash);

        Ok(self.inner.db.write(batch)?)
    }

    fn discard_changeset(&self) {
        *(self.inner.storage_pending_data.write().unwrap()) = None;
    }
}

// ------------------------------- state storage -------------------------------

#[derive(Clone)]
pub struct StateStorage {
    inner: Arc<DiskDbLiteInner>,
    cf_name: &'static str,
}

impl Storage for StateStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.inner
            .db
            .get_cf(&cf_handle(&self.inner.db, self.cf_name), key)
            .unwrap_or_else(|err| {
                panic!("failed to read from DB! cf: {}, err: {}", self.cf_name, err);
            })
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = grug_types::Record> + 'a> {
        let opts = new_read_options(min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_handle(&self.inner.db, self.cf_name), opts, mode)
            .map(|item| {
                let (k, v) = item.unwrap_or_else(|err| {
                    panic!(
                        "failed to iterate in DB! cf: {}, err: {}",
                        self.cf_name, err
                    );
                });
                (k.to_vec(), v.to_vec())
            });

        Box::new(iter)
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let opts = new_read_options(min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_handle(&self.inner.db, self.cf_name), opts, mode)
            .map(|item| {
                let (k, _) = item.unwrap_or_else(|err| {
                    panic!(
                        "failed to iterate in DB! cf: {}, err: {}",
                        self.cf_name, err
                    );
                });
                k.to_vec()
            });

        Box::new(iter)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let opts = new_read_options(min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_handle(&self.inner.db, self.cf_name), opts, mode)
            .map(|item| {
                let (_, v) = item.unwrap_or_else(|err| {
                    panic!(
                        "failed to iterate in DB! cf: {}, err: {}",
                        self.cf_name, err
                    );
                });
                v.to_vec()
            });

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

// ------------------------------- state consensus -------------------------------

/// Unlike [`StateStorage`], we allow write operations as it is not always necessary to use a [`grug_types::Buffer`]
#[derive(Clone)]
pub struct StateConsensus {
    inner: Arc<DiskDbLiteInner>,
    cf_name: &'static str,
}

impl ConsensusStorage for StateConsensus {}

impl Storage for StateConsensus {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.inner
            .db
            .get_cf(&cf_handle(&self.inner.db, self.cf_name), key)
            .unwrap_or_else(|err| {
                panic!("failed to read from DB! cf: {}, err: {}", self.cf_name, err);
            })
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = grug_types::Record> + 'a> {
        let opts = new_read_options(min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_handle(&self.inner.db, self.cf_name), opts, mode)
            .map(|item| {
                let (k, v) = item.unwrap_or_else(|err| {
                    panic!(
                        "failed to iterate in DB! cf: {}, err: {}",
                        self.cf_name, err
                    );
                });
                (k.to_vec(), v.to_vec())
            });

        Box::new(iter)
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let opts = new_read_options(min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_handle(&self.inner.db, self.cf_name), opts, mode)
            .map(|item| {
                let (k, _) = item.unwrap_or_else(|err| {
                    panic!(
                        "failed to iterate in DB! cf: {}, err: {}",
                        self.cf_name, err
                    );
                });
                k.to_vec()
            });

        Box::new(iter)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let opts = new_read_options(min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_handle(&self.inner.db, self.cf_name), opts, mode)
            .map(|item| {
                let (_, v) = item.unwrap_or_else(|err| {
                    panic!(
                        "failed to iterate in DB! cf: {}, err: {}",
                        self.cf_name, err
                    );
                });
                v.to_vec()
            });

        Box::new(iter)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.inner
            .db
            .put_cf(&cf_handle(&self.inner.db, self.cf_name), key, value)
            .unwrap_or_else(|err| {
                panic!("failed to write to state consensus: {err}");
            });
    }

    fn remove(&mut self, key: &[u8]) {
        self.inner
            .db
            .delete_cf(&cf_handle(&self.inner.db, self.cf_name), key)
            .unwrap_or_else(|err| {
                panic!("failed to delete from state consensus: {err}");
            });
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        for k in self.scan_keys(min, max, Order::Ascending) {
            self.inner
                .db
                .delete_cf(&cf_handle(&self.inner.db, self.cf_name), &k)
                .unwrap_or_else(|err| {
                    panic!("failed to delete from state consensus: {err}");
                });
        }
    }
}

// ---------------------------------- helpers ----------------------------------

fn into_iterator_mode(order: Order) -> IteratorMode<'static> {
    match order {
        Order::Ascending => IteratorMode::Start,
        Order::Descending => IteratorMode::End,
    }
}

// TODO: rocksdb tuning? see:
// https://github.com/sei-protocol/sei-db/blob/main/ss/rocksdb/opts.go#L29-L65
// https://github.com/turbofish-org/merk/blob/develop/src/merk/mod.rs#L84-L102
fn new_db_options() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts
}

fn new_read_options(
    iterate_lower_bound: Option<&[u8]>,
    iterate_upper_bound: Option<&[u8]>,
) -> ReadOptions {
    let mut opts = ReadOptions::default();
    if let Some(bound) = iterate_lower_bound {
        opts.set_iterate_lower_bound(bound);
    }
    if let Some(bound) = iterate_upper_bound {
        opts.set_iterate_upper_bound(bound);
    }
    opts
}

fn cf_handle<'a>(
    db: &'a DBWithThreadMode<MultiThreaded>,
    name: &'static str,
) -> Arc<BoundColumnFamily<'a>> {
    db.cf_handle(name).unwrap_or_else(|| {
        panic!("failed to create handle for `{name}` column family");
    })
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug_types::hash, temp_rocksdb::TempDataDir};

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
        let db = DiskDbLite::open(&path).unwrap();

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
}
