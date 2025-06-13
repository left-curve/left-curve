use {
    crate::{DbError, DbResult, VersionedMap},
    grug_app::{ConsensusStorage, Db},
    grug_jmt::MerkleTree,
    grug_types::{Batch, Buffer, Hash256, HashExt, Op, Order, Proof, Record, Storage},
    std::{
        collections::{BTreeMap, HashMap},
        ops::Bound,
        sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    },
};

const MERKLE_TREE: MerkleTree = MerkleTree::new_default();

struct ChangeSet {
    version: u64,
    state_commitment: Batch,
    state_storage: Batch,
}

struct MemDbInner {
    /// Version of the DB. Initilialized to `None` when the DB instance is
    /// created. Set of 0 the first time a batch of data is committed, and
    /// incremented by 1 each time afterwards.
    latest_version: Option<u64>,
    /// A key-value store backing the Merkle tree.
    ///
    /// A HashMap is chosen over BTreeMap because our Merkle tree implementation
    /// does not need to iterate raw keys in this store.
    state_commitment: HashMap<Vec<u8>, Vec<u8>>,
    /// A versioned key-value storage: key => (version => value)
    state_storage: VersionedMap<Vec<u8>, Vec<u8>>,
    /// Uncommitted changes
    changeset: Option<ChangeSet>,
    /// A key-value store backing the consensus.
    consensus: BTreeMap<Vec<u8>, Vec<u8>>,
}

pub struct MemDb {
    inner: Arc<RwLock<MemDbInner>>,
}

impl MemDb {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MemDbInner {
                latest_version: None,
                state_commitment: HashMap::new(),
                state_storage: VersionedMap::new(),
                changeset: None,
                consensus: BTreeMap::new(),
            })),
        }
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

impl Default for MemDb {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MemDb {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Db for MemDb {
    type Consensus = Consensus;
    type Error = DbError;
    type Proof = Proof;
    type StateCommitment = StateCommitment;
    type StateStorage = StateStorage;

    fn state_commitment(&self) -> StateCommitment {
        StateCommitment { db: self.clone() }
    }

    fn state_storage(&self, version: Option<u64>) -> DbResult<StateStorage> {
        Ok(StateStorage {
            db: self.clone(),
            version: version.unwrap_or_else(|| self.latest_version().unwrap_or(0)),
        })
    }

    fn consensus(&self) -> Self::Consensus {
        Consensus { db: self.clone() }
    }

    fn latest_version(&self) -> Option<u64> {
        self.with_read(|inner| inner.latest_version)
    }

    fn root_hash(&self, version: Option<u64>) -> DbResult<Option<Hash256>> {
        let version = version.unwrap_or_else(|| self.latest_version().unwrap_or(0));
        Ok(MERKLE_TREE.root_hash(&self.state_commitment(), version)?)
    }

    fn prove(&self, key: &[u8], version: Option<u64>) -> DbResult<Proof> {
        let version = version.unwrap_or_else(|| self.latest_version().unwrap_or(0));
        Ok(MERKLE_TREE.prove(&self.state_commitment(), key.hash256(), version)?)
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
        let (new_version, root_hash, changeset) = self.with_read(|inner| {
            if inner.changeset.is_some() {
                return Err(DbError::ChangeSetAlreadySet);
            }

            let (old_version, new_version) = match self.latest_version() {
                Some(v) => (v, v + 1),
                None => (0, 0),
            };

            let mut cache = Buffer::new(self.state_commitment(), None);
            let root_hash = MERKLE_TREE.apply_raw(&mut cache, old_version, new_version, &batch)?;
            let (_, changeset) = cache.disassemble();

            Ok((new_version, root_hash, changeset))
        })?;

        self.with_write(|mut inner| {
            inner.changeset = Some(ChangeSet {
                version: new_version,
                state_commitment: changeset,
                state_storage: batch,
            });
        });

        Ok((new_version, root_hash))
    }

    fn commit(&self) -> DbResult<()> {
        self.with_write(|mut inner| {
            let changeset = inner.changeset.take().ok_or(DbError::ChangeSetNotSet)?;

            // Update the version
            inner.latest_version = Some(changeset.version);

            // Write changes to state commitment
            for (key, op) in changeset.state_commitment {
                if let Op::Insert(value) = op {
                    inner.state_commitment.insert(key, value);
                } else {
                    inner.state_commitment.remove(&key);
                }
            }

            // Write changes to state storage
            inner.state_storage.write_batch(changeset.state_storage);

            Ok(())
        })
    }

    fn discard_changeset(&self) {
        self.with_write(|mut inner| {
            inner.changeset = None;
        })
    }
}

// ----------------------------- state commitment ------------------------------

#[derive(Clone)]
pub struct StateCommitment {
    db: MemDb,
}

impl Storage for StateCommitment {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db
            .with_read(|inner| inner.state_commitment.get(key).cloned())
    }

    fn scan<'a>(
        &'a self,
        _min: Option<&[u8]>,
        _max: Option<&[u8]>,
        _order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        unimplemented!("this isn't used by the Merkle tree");
    }

    fn scan_keys<'a>(
        &'a self,
        _min: Option<&[u8]>,
        _max: Option<&[u8]>,
        _order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        unimplemented!("this isn't used by the Merkle tree");
    }

    fn scan_values<'a>(
        &'a self,
        _min: Option<&[u8]>,
        _max: Option<&[u8]>,
        _order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        unimplemented!("this isn't used by the Merkle tree");
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

// ------------------------------- state storage -------------------------------

#[derive(Clone)]
pub struct StateStorage {
    db: MemDb,
    version: u64,
}

impl Storage for StateStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db
            .with_read(|inner| inner.state_storage.get(key, self.version).cloned())
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
                .range::<_, [u8]>((min, max), self.version)
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

// ------------------------------ consensus storage ----------------------------

#[derive(Clone)]
pub struct Consensus {
    db: MemDb,
}

macro_rules! range_bounds {
    ($min:ident, $max:ident) => {{
        // `BTreeMap::range` panics if
        // 1. start > end, or
        // 2. start == end and both are exclusive
        // For us, since we interpret min as inclusive and max as exclusive,
        // only the 1st case apply. However, we don't want to panic, we just
        // return an empty iterator.
        if let (Some(min), Some(max)) = ($min, $max) {
            if min > max {
                return Box::new(std::iter::empty());
            }
        }

        // Min is inclusive, max is exclusive.
        let min = $min.map_or(Bound::Unbounded, |bytes| Bound::Included(bytes.to_vec()));
        let max = $max.map_or(Bound::Unbounded, |bytes| Bound::Excluded(bytes.to_vec()));

        (min, max)
    }};
}

impl Storage for Consensus {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db.with_read(|inner| inner.consensus.get(key).cloned())
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let bounds = range_bounds!(min, max);
        let vec = self.db.with_read(|inner| {
            // Here we must collect the iterator into a `Vec`, because the
            // iterator only lives as longa as the read lock, which goes out of
            // scope at the end of the function.
            inner
                .consensus
                .range(bounds)
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
        let bounds = range_bounds!(min, max);
        let vec = self.db.with_read(|inner| {
            // Here we must collect the iterator into a `Vec`, because the
            // iterator only lives as longa as the read lock, which goes out of
            // scope at the end of the function.
            inner
                .consensus
                .range(bounds)
                .map(|(k, _)| k.clone())
                .collect::<Vec<_>>()
        });
        match order {
            Order::Ascending => Box::new(vec.into_iter()),
            Order::Descending => Box::new(vec.into_iter().rev()),
        }
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let bounds = range_bounds!(min, max);
        let vec = self.db.with_read(|inner| {
            // Here we must collect the iterator into a `Vec`, because the
            // iterator only lives as longa as the read lock, which goes out of
            // scope at the end of the function.
            inner
                .consensus
                .range(bounds)
                .map(|(_, v)| v.clone())
                .collect::<Vec<_>>()
        });
        match order {
            Order::Ascending => Box::new(vec.into_iter()),
            Order::Descending => Box::new(vec.into_iter().rev()),
        }
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.db
            .with_write(|mut inner| inner.consensus.insert(key.to_vec(), value.to_vec()));
    }

    fn remove(&mut self, key: &[u8]) {
        self.db.with_write(|mut inner| inner.consensus.remove(key));
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        self.db.with_write(|mut inner| {
            inner.consensus.retain(|k, _| {
                if let Some(min) = min {
                    if k.as_slice() < min {
                        return true;
                    }
                }

                if let Some(max) = max {
                    if max <= k.as_slice() {
                        return true;
                    }
                }

                false
            })
        });
    }
}

impl ConsensusStorage for Consensus {}
