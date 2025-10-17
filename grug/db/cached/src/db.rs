use {
    crate::error::{DbError, DbResult},
    error_backtrace::Backtraceable,
    grug_app::Db,
    grug_types::{Op, Order, Proof, Record, Shared, Storage, btree_map},
    std::{collections::BTreeMap, fmt, ops::Bound, sync::Arc},
};

pub struct CachedDb<DB> {
    db: DB,
    inner: Shared<CachedDbInner>,
    cache_size: usize,
}

struct CachedDbInner {
    memory: BTreeMap<u64, Arc<BTreeMap<Vec<u8>, Vec<u8>>>>,
    next_pending: Option<BTreeMap<Vec<u8>, Vec<u8>>>,
    next_version: Option<u64>,
}

impl<DB> CachedDb<DB>
where
    DB: Db,
    DB::Error: fmt::Display + Backtraceable,
{
    pub fn new(db: DB, cache_size: usize) -> DbResult<Self, DB::Error> {
        let latest_version = db.latest_version();

        // Initialize the memory with the last version's state storage of the db.
        let memory = if let Some(last_version) = latest_version {
            let storage = db.state_storage(Some(last_version))?;
            let last_memory = storage
                .scan(None, None, Order::Ascending)
                .collect::<BTreeMap<_, _>>();

            btree_map!(last_version => Arc::new(last_memory))
        } else {
            BTreeMap::new()
        };

        Ok(Self {
            db,
            inner: Shared::new(CachedDbInner {
                memory,
                next_pending: None,
                next_version: None,
            }),
            cache_size,
        })
    }

    fn try_remove_oldest(&self) {
        self.inner.write_with(|mut inner| {
            // inner.memory.len() could be at most self.cache_size + 1 elements
            if inner.memory.len() > self.cache_size {
                inner.memory.pop_first();
            }
        })
    }

    fn get_cache(&self, version: u64) -> Option<Arc<BTreeMap<Vec<u8>, Vec<u8>>>> {
        self.inner
            .read_with(|inner| inner.memory.get(&version).cloned())
    }
}

impl<DB> Db for CachedDb<DB>
where
    DB: Db + Clone + 'static,
    DB::Error: std::fmt::Display + Backtraceable,
{
    type Error = DbError<DB::Error>;
    type Proof = Proof;
    type StateCommitment = StateStorage<DB>;
    type StateStorage = StateStorage<DB>;

    fn state_commitment(&self) -> Self::StateCommitment {
        unimplemented!("`HybridDb` does not support state commitment");
    }

    fn state_storage(&self, version: Option<u64>) -> Result<Self::StateStorage, Self::Error> {
        let requested = if let Some(requested) = version {
            Some(requested)
        } else {
            self.db.latest_version()
        };

        if let Some(requested) = requested {
            match self.get_cache(requested) {
                Some(cache) => Ok(StateStorage::Cached(cache)),
                // if the cache is not found, it means the version is not in the memory, so we need to read it from the db
                None => Ok(StateStorage::Db(self.db.state_storage(Some(requested))?)),
            }
        } else {
            Ok(StateStorage::Cached(Default::default()))
        }
    }

    fn latest_version(&self) -> Option<u64> {
        self.db.latest_version()
    }

    fn root_hash(&self, version: Option<u64>) -> Result<Option<grug_types::Hash256>, Self::Error> {
        Ok(self.db.root_hash(version)?)
    }

    fn prove(&self, _key: &[u8], _version: Option<u64>) -> Result<Self::Proof, Self::Error> {
        Err(DbError::proof_unsupported())
    }

    fn flush_but_not_commit(
        &self,
        batch: grug_types::Batch,
    ) -> Result<(u64, Option<grug_types::Hash256>), Self::Error> {
        let current_version = self.db.latest_version();
        let (next_version, root_hash) = self.db.flush_and_commit(batch.clone())?;

        // Prepare the memory for the next version.
        let mut map = if let Some(current_version) = current_version {
            (*self
                .get_cache(current_version)
                .ok_or(DbError::version_not_in_memory(current_version))?)
            .clone()
        } else {
            BTreeMap::new()
        };

        for (key, value) in batch {
            if let Op::Insert(value) = value {
                map.insert(key, value);
            } else {
                map.remove(&key);
            }
        }

        self.inner.write_with(|mut inner| {
            inner.next_pending = Some(map);
            inner.next_version = Some(next_version);
        });

        Ok((next_version, root_hash))
    }

    fn commit(&self) -> Result<(), Self::Error> {
        self.inner.write_with(|mut inner| {
            let next_pending = inner
                .next_pending
                .take()
                .ok_or(DbError::next_pending_not_set())?;
            let next_version = inner
                .next_version
                .take()
                .ok_or(DbError::next_version_not_set())?;

            inner.memory.insert(next_version, Arc::new(next_pending));
            inner.next_pending = None;
            inner.next_version = None;
            Ok::<_, DbError<DB::Error>>(())
        })?;

        self.db.commit()?;

        self.try_remove_oldest();

        Ok(())
    }
}

#[derive(Clone)]
pub enum StateStorage<DB: Db> {
    Cached(Arc<BTreeMap<Vec<u8>, Vec<u8>>>),
    Db(DB::StateStorage),
}

impl<DB> Storage for StateStorage<DB>
where
    DB: Db + Clone,
{
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        match self {
            StateStorage::Cached(inner) => inner.get(key).cloned(),
            StateStorage::Db(inner) => inner.read(key),
        }
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        match self {
            StateStorage::Cached(cache) => {
                let min = min.map_or(Bound::Unbounded, Bound::Included);
                let max = max.map_or(Bound::Unbounded, Bound::Excluded);
                let iter = cache
                    .range::<[u8], _>((min, max))
                    .map(|(k, v)| (k.clone(), v.clone()));
                match order {
                    Order::Ascending => Box::new(iter),
                    Order::Descending => Box::new(iter.rev()),
                }
            },
            StateStorage::Db(db) => db.scan(min, max, order),
        }
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        match self {
            StateStorage::Cached(_) => Box::new(self.scan(min, max, order).map(|(k, _)| k)),
            StateStorage::Db(db) => db.scan_keys(min, max, order),
        }
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        match self {
            StateStorage::Cached(_) => Box::new(self.scan(min, max, order).map(|(_, v)| v)),
            StateStorage::Db(db) => db.scan_values(min, max, order),
        }
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
