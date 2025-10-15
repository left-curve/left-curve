use {
    crate::{DbError, DbResult, digest::batch_hash},
    grug_app::Db,
    grug_types::{Batch, Hash256, Op, Order, Proof, Record, Storage},
    std::{
        collections::BTreeMap,
        fs,
        ops::Bound,
        path::Path,
        sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    },
};
#[cfg(feature = "dump")]
use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{BorshDeExt, BorshSerExt},
};

#[cfg_attr(feature = "dump", derive(BorshSerialize, BorshDeserialize))]
struct ChangeSet {
    version: u64,
    state_storage: Batch,
}

#[cfg_attr(feature = "dump", derive(BorshSerialize, BorshDeserialize))]
struct MemDbInner {
    /// Version of the DB. Initilialized to `None` when the DB instance is
    /// created. Set of 0 the first time a batch of data is committed, and
    /// incremented by 1 each time afterwards.
    latest_version: Option<u64>,
    /// The root hash of the last committed batch.
    last_root_hash: Option<Hash256>,
    /// A key-value storage: key => value
    state_storage: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Uncommitted changes
    changeset: Option<ChangeSet>,
}

pub struct MemDbLite {
    inner: Arc<RwLock<MemDbInner>>,
}

impl MemDbLite {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MemDbInner {
                latest_version: None,
                last_root_hash: None,
                state_storage: BTreeMap::new(),
                changeset: None,
            })),
        }
    }

    /// Get direct immutable access to the state storage.
    pub fn with_state_storage<C, T>(&self, callback: C) -> T
    where
        C: FnOnce(&dyn Storage) -> T,
    {
        self.with_write(|inner| callback(&inner.state_storage))
    }

    /// Get direct mutable access to the state storage. Only intended for testing
    /// and debugging purposes.
    pub fn with_state_storage_mut<C, T>(&self, callback: C) -> T
    where
        C: FnOnce(&mut dyn Storage) -> T,
    {
        self.with_write(|mut inner| callback(&mut inner.state_storage))
    }

    fn with_read<C, T>(&self, callback: C) -> T
    where
        C: FnOnce(RwLockReadGuard<MemDbInner>) -> T,
    {
        let lock = self.inner.read().unwrap_or_else(|err| {
            panic!("MemDb is poisoned: {err:?}");
        });
        callback(lock)
    }

    fn with_write<C, T>(&self, callback: C) -> T
    where
        C: FnOnce(RwLockWriteGuard<MemDbInner>) -> T,
    {
        let lock = self.inner.write().unwrap_or_else(|err| {
            panic!("MemDb is poisoned: {err:?}");
        });
        callback(lock)
    }
}

#[cfg(feature = "dump")]
impl MemDbLite {
    /// Dump the database to a file.
    pub fn dump<P>(&self, path: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let bytes = self.with_read(|inner| inner.to_borsh_vec())?;

        Ok(fs::write(path, bytes)?)
    }

    /// Recover the database to a file.
    pub fn recover<P>(path: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path>,
    {
        let bytes = fs::read(path)?;
        let inner = bytes.deserialize_borsh()?;

        Ok(Self {
            inner: Arc::new(RwLock::new(inner)),
        })
    }
}

impl Default for MemDbLite {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MemDbLite {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Db for MemDbLite {
    type Error = DbError;
    type Proof = Proof;
    type StateCommitment = StateStorage;
    type StateStorage = StateStorage;

    fn state_commitment(&self) -> Self::StateCommitment {
        unimplemented!("`MemDbLite` does not support state commitment");
    }

    fn state_storage(&self, version: Option<u64>) -> DbResult<StateStorage> {
        if let Some(requested) = version {
            let db_version = self.latest_version().unwrap_or(0);
            if requested != db_version {
                return Err(DbError::incorrect_version(db_version, requested));
            }
        }
        Ok(StateStorage { db: self.clone() })
    }

    fn latest_version(&self) -> Option<u64> {
        self.with_read(|inner| inner.latest_version)
    }

    fn root_hash(&self, version: Option<u64>) -> DbResult<Option<Hash256>> {
        self.with_read(|inner| {
            if inner.latest_version != version {
                return Ok(None);
            }

            Ok(inner.last_root_hash)
        })
    }

    fn prove(&self, _key: &[u8], _version: Option<u64>) -> DbResult<Proof> {
        Err(DbError::proof_unsupported())
    }

    // Note on implementing this function: We must make sure that we don't
    // attempt to lock the DB (either read or write) inside the `with_write`
    // callback. Doing so will result in error:
    //
    // > rwlock read lock would result in deadlock
    //
    // The best way to avoid this is to do everything that requires a read lock
    // first (using a `with_read` callback) and do everything that requires a
    // write lock in the end (using a `with_write` callback).
    fn flush_but_not_commit(&self, batch: Batch) -> DbResult<(u64, Option<Hash256>)> {
        let (version, hash) = self.with_write(|inner| {
            if inner.changeset.is_some() {
                return Err(DbError::change_set_already_set());
            }

            let version = inner.latest_version.map(|v| v + 1).unwrap_or(0);

            // Since the Lite DB doesn't Merklize the state, we generate a hash based
            // on the changeset.
            let hash = batch_hash(&batch);

            Ok((version, hash))
        })?;

        self.with_write(|mut inner| {
            inner.changeset = Some(ChangeSet {
                version,
                state_storage: batch,
            });
        });

        Ok((version, Some(hash)))
    }

    fn commit(&self) -> DbResult<()> {
        self.with_write(|mut inner| {
            let changeset = inner
                .changeset
                .take()
                .ok_or(DbError::change_set_not_set())?;

            // Update the version
            inner.latest_version = Some(changeset.version);

            // Write changes to state storage
            {
                for (key, op) in changeset.state_storage {
                    if let Op::Insert(value) = op {
                        inner.state_storage.insert(key, value);
                    } else {
                        inner.state_storage.remove(&key);
                    }
                }
            }

            Ok(())
        })
    }
}

// ------------------------------- state storage -------------------------------

#[derive(Clone)]
pub struct StateStorage {
    db: MemDbLite,
}

impl Storage for StateStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db
            .with_read(|inner| inner.state_storage.get(key).cloned())
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let min = min.map_or(Bound::Unbounded, Bound::Included);
        let max = max.map_or(Bound::Unbounded, Bound::Excluded);
        let vec = self.db.with_read(|inner| {
            // Here we must collect the iterator into a `Vec`, because the
            // iterator only lives as longa as the read lock, which goes out of
            // scope at the end of the function.
            inner
                .state_storage
                .range::<[u8], _>((min, max))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<_>>()
        });
        match order {
            Order::Ascending => Box::new(vec.into_iter()),
            Order::Descending => Box::new(vec.into_iter().rev()),
        }
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        // Here we take the approach of iterating both the keys and the values,
        // and simply discard the values. Apparently this isn't efficient since
        // in `scan` we clone the values for no purpose. This said, db/memory is
        // for running tests only, so this is ok.
        let iter = self.scan(min, max, order).map(|(k, _)| k);
        Box::new(iter)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let iter = self.scan(min, max, order).map(|(_, v)| v);
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
