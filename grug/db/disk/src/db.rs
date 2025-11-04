#[cfg(feature = "metrics")]
use grug_types::MetricsIterExt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use {
    crate::{DbError, DbResult, U64Comparator, U64Timestamp},
    grug_app::{Commitment, Db},
    grug_types::{Batch, Buffer, Hash256, HashExt, Op, Order, Record, Storage},
    rocksdb::{
        BoundColumnFamily, DBWithThreadMode, IteratorMode, MultiThreaded, Options, ReadOptions,
        WriteBatch,
    },
    std::{
        marker::PhantomData,
        path::Path,
        sync::{Arc, RwLock},
    },
};

/// We use three column families (CFs) for storing data.
/// The default family is used for metadata. Currently the only metadata we have
/// is the latest version.
pub const CF_NAME_DEFAULT: &str = "default";

/// The preimage column family maps key hashes to raw keys. This is necessary
/// for generating ICS-23 compatible Merkle proofs.
#[cfg(feature = "ibc")]
pub const CF_NAME_PREIMAGES: &str = "preimages";

/// The state commitment (SC) family stores Merkle tree nodes, which hold hashed
/// key-value pair data. We use this CF for deriving the Merkle root hash for the
/// state (used in consensus) and generating Merkle proofs (used in light clients).
pub const CF_NAME_STATE_COMMITMENT: &str = "state_commitment";

/// The state storage (SS) family stores raw, prehash key-value pair data.
/// When performing normal read/write/remove/scan interactions, we use this CF.
///
/// It also utilize RocksDB's timestamping feature to provide historical state
/// access, necessary for archive nodes:
/// https://github.com/facebook/rocksdb/wiki/User-defined-Timestamp
///
/// Unfortunately the Rust API for RocksDB does not support timestamping,
/// we have to add it in. Our fork is here, under the `0.21.0-cw` branch:
/// https://github.com/left-curve/rust-rocksdb/tree/v0.21.0-cw
pub const CF_NAME_STATE_STORAGE: &str = "state_storage";

/// Storage key for the latest version.
pub const LATEST_VERSION_KEY: &[u8] = b"latest_version";

/// Storage key for the oldest version.
pub const OLDEST_VERSION_KEY: &[u8] = b"oldest_version";

#[cfg(feature = "metrics")]
pub const DISK_DB_LABEL: &str = "grug.db.disk.duration";

/// Configurations related to the disk DB.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    // The interval of auto-pruning (i.e. perform a pruning every X versions).
    // 0 means no auto-pruning.
    pub prune_interval: u64,
    /// At each auto-pruning, how many recent versions to keep.
    pub prune_keep_recent: u64,
    /// Whether to force an immediate database compaction when auto-pruning.
    pub prune_force_compact: bool,
}

/// The base storage primitive.
///
/// Its main feature is the separation of state storage (SS) and state commitment
/// (SC). Specifically, SS stores _prehash_ key-value pairs (KV pairs), while SC
/// stores _hashed_ KV pairs in a Merkle tree data structure. SS is used for
/// normal read/write access to the state, while SC is used for deriving Merkle
/// root hashes for the state (used by nodes to reach consensus) and generate
/// Merkle proofs for data in the state (used by light clients, such as in iBC).
///
/// The separation of SS and SC was first conceived by the Cosmos SDK core team
/// in ADR-040:
/// <https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-040-storage-and-smt-state-commitments.md>
/// and later refined in ADR-065:
/// <https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-065-store-v2.md>.
/// It was first pushed to production use by Sei, marketed as SeiDB:
/// <https://blog.sei.io/sei-db-the-numbers/>
///
/// Our design mostly resembles Sei's with the differences being that:
/// - we use a binary Jellyfish Merkle tree (JMT) instead of IAVL;
/// - we store JMT data in a RocksDB instance, instead of using memory map (mmap);
/// - we don't have asynchronous commit;
/// - we don't store snapshots or use a WAL to recover the latest state.
///
/// These differences are not because we don't agree with Sei's approach...
/// it's just because we're having here is sort of a quick hack and we don't
/// have time to look into those advanced features yet. We will keep experimenting
/// and maybe our implementation will converge with Sei's some time later.
pub struct DiskDb<T> {
    pub(crate) inner: Arc<DiskDbInner>,
    cfg: Config,
    _commitment: PhantomData<T>,
}

#[derive(Debug)]
pub(crate) struct DiskDbInner {
    pub db: DBWithThreadMode<MultiThreaded>,
    // Data that are ready to be persisted to the physical database.
    // Ideally we want to just use a `rocksdb::WriteBatch` here, but it's not
    // thread-safe.
    pending_data: RwLock<Option<PendingData>>,
}

#[derive(Debug)]
pub(crate) struct PendingData {
    version: u64,
    state_commitment: Batch,
    state_storage: Batch,
}

impl<T> DiskDb<T> {
    /// Create a DiskDb instance by opening a physical RocksDB instance, using
    /// the default configurations.
    pub fn open<P>(data_dir: P) -> DbResult<Self>
    where
        P: AsRef<Path>,
    {
        Self::open_with_cfg(data_dir, Config::default())
    }

    /// Create a DiskDb instance by opening a physical RocksDB instance.
    pub fn open_with_cfg<P>(data_dir: P, cfg: Config) -> DbResult<Self>
    where
        P: AsRef<Path>,
    {
        // Note: For default and state commitment CFs, don't enable timestamping;
        // for state storage column family, enable timestamping.
        let db = DBWithThreadMode::open_cf_with_opts(&new_db_options(), data_dir, [
            (CF_NAME_DEFAULT, Options::default()),
            #[cfg(feature = "ibc")]
            (CF_NAME_PREIMAGES, new_cf_options_with_ts()),
            (CF_NAME_STATE_STORAGE, new_cf_options_with_ts()),
            (CF_NAME_STATE_COMMITMENT, Options::default()),
        ])?;

        Ok(Self {
            inner: Arc::new(DiskDbInner {
                db,
                pending_data: RwLock::new(None),
            }),
            cfg,
            _commitment: PhantomData,
        })
    }
}

impl<T> Clone for DiskDb<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            cfg: self.cfg.clone(),
            _commitment: PhantomData,
        }
    }
}

impl<T> Db for DiskDb<T>
where
    T: Commitment,
{
    type Error = DbError;
    type Proof = T::Proof;
    type StateCommitment = StateCommitment;
    type StateStorage = StateStorage;

    fn state_commitment(&self) -> StateCommitment {
        StateCommitment {
            inner: Arc::clone(&self.inner),
        }
    }

    fn state_storage_with_comment(
        &self,
        version: Option<u64>,
        comment: &'static str,
    ) -> DbResult<StateStorage> {
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
            inner: Arc::clone(&self.inner),
            version,
            comment,
        })
    }

    fn latest_version(&self) -> Option<u64> {
        let cf = cf_default(&self.inner.db);
        let bytes = self
            .inner
            .db
            .get_cf(&cf, LATEST_VERSION_KEY)
            .unwrap_or_else(|err| {
                panic!("failed to read from default column family: {err}");
            })?;
        let array = bytes.try_into().unwrap_or_else(|bytes: Vec<u8>| {
            panic!(
                "latest version is of incorrect byte length: {}",
                bytes.len()
            );
        });
        Some(u64::from_le_bytes(array))
    }

    fn oldest_version(&self) -> Option<u64> {
        let cf = cf_default(&self.inner.db);
        let bytes = self
            .inner
            .db
            .get_cf(&cf, OLDEST_VERSION_KEY)
            .unwrap_or_else(|err| {
                panic!("failed to read from default column family: {err}");
            })?;
        let array = bytes.try_into().unwrap_or_else(|bytes: Vec<u8>| {
            panic!(
                "oldest version is of incorrect byte length: {}",
                bytes.len()
            );
        });
        Some(u64::from_le_bytes(array))
    }

    fn root_hash(&self, version: Option<u64>) -> DbResult<Option<Hash256>> {
        let version = version.unwrap_or_else(|| self.latest_version().unwrap_or(0));
        Ok(T::root_hash(&self.state_commitment(), version)?)
    }

    fn prove(&self, key: &[u8], version: Option<u64>) -> DbResult<Self::Proof> {
        let version = version.unwrap_or_else(|| self.latest_version().unwrap_or(0));
        Ok(T::prove(&self.state_commitment(), key.hash256(), version)?)
    }

    fn flush_but_not_commit(&self, batch: Batch) -> DbResult<(u64, Option<Hash256>)> {
        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        // A write batch must not already exist. If it does, it means a batch
        // has been flushed, but not committed, then a next batch is flusehd,
        // which indicates some error in the ABCI app's logic.
        if self.inner.pending_data.read()?.is_some() {
            return Err(DbError::pending_data_already_set());
        }

        let (old_version, new_version) = match self.latest_version() {
            // An old version exist.
            // Set the new version to be the old version plus one
            Some(v) => (v, v + 1),
            // The old version doesn't exist. This means not a first batch has
            // been flushed yet. In this case, we set the new version to be zero.
            // This is necessary to ensure that DB version always matches the
            // block height.
            None => (0, 0),
        };

        // Commit hashed KVs to state commitment.
        // The DB writes here are kept in the in-memory `PendingData`.
        let mut buffer = Buffer::new(
            self.state_commitment(),
            None,
            "disk_db_state_commitment_flush_but_not_commit",
        );

        let root_hash = T::apply(&mut buffer, old_version, new_version, &batch)?;
        let (_, pending) = buffer.disassemble();

        *(self.inner.pending_data.write()?) = Some(PendingData {
            version: new_version,
            state_commitment: pending,
            state_storage: batch,
        });

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LABEL, "operation" => "flush_but_not_commit")
                .record(duration.elapsed().as_secs_f64());
        }

        Ok((new_version, root_hash))
    }

    fn commit(&self) -> DbResult<()> {
        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        let pending = self
            .inner
            .pending_data
            .write()?
            .take()
            .ok_or(DbError::pending_data_not_set())?;
        let mut batch = WriteBatch::default();
        let ts = U64Timestamp::from(pending.version);

        // Set the new version (note: use little endian)
        let cf = cf_default(&self.inner.db);
        batch.put_cf(&cf, LATEST_VERSION_KEY, pending.version.to_le_bytes());

        // Writes in state commitment
        let cf = cf_state_commitment(&self.inner.db);
        for (key, op) in pending.state_commitment {
            if let Op::Insert(value) = op {
                batch.put_cf(&cf, key, value);
            } else {
                batch.delete_cf(&cf, key);
            }
        }

        // Writes in preimages (note: don't forget timestamping, and deleting
        // key hashes that are deleted in state storage - see Zellic audut).
        // This is only necessary if the `ibc` feature is enabled.
        #[cfg(feature = "ibc")]
        {
            let cf = cf_preimages(&self.inner.db);
            for (key, op) in &pending.state_storage {
                if let Op::Insert(_) = op {
                    batch.put_cf_with_ts(&cf, key.hash256(), ts, key);
                } else {
                    batch.delete_cf_with_ts(&cf, key.hash256(), ts);
                }
            }
        }

        // Writes in state storage (note: don't forget timestamping)
        let cf = cf_state_storage(&self.inner.db);
        for (key, op) in pending.state_storage {
            if let Op::Insert(value) = op {
                batch.put_cf_with_ts(&cf, key, ts, value);
            } else {
                batch.delete_cf_with_ts(&cf, key, ts);
            }
        }

        self.inner.db.write(batch)?;

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LABEL, "operation" => "commit")
                .record(duration.elapsed().as_secs_f64());
        }

        // Perform auto-pruning if the configured interval is reached.
        if self.cfg.prune_interval != 0
            && pending.version % self.cfg.prune_interval == 0
            && pending.version > self.cfg.prune_keep_recent
        {
            self.prune(pending.version - self.cfg.prune_keep_recent)?;

            if self.cfg.prune_force_compact {
                self.compact();
            }
        }

        Ok(())
    }

    fn prune(&self, up_to_version: u64) -> DbResult<()> {
        #[cfg(feature = "tracing")]
        {
            tracing::info!(up_to_version, "Pruning database");
        }

        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        let ts = U64Timestamp::from(up_to_version);

        // Prune state storage.
        //
        // We do this by increase the state storage column family's
        // `full_history_ts_low` value, as in SeiDB:
        // <https://github.com/sei-protocol/sei-db/blob/v0.0.41/ss/rocksdb/db.go#L186-L206>
        //
        // Note, this does _not_ incur an immediate full compaction, i.e. this
        // performs a lazy prune. Future compactions will honor the increased
        // `full_history_ts_low` and trim history when possible.
        let cf = cf_state_storage(&self.inner.db);
        self.inner.db.increase_full_history_ts_low(&cf, ts)?;

        // Same for preimages.
        #[cfg(feature = "ibc")]
        {
            let cf = cf_preimages(&self.inner.db);
            self.inner.db.increase_full_history_ts_low(&cf, ts)?;
        }

        // Prune state commitment.
        let mut buffer = Buffer::new(
            self.state_commitment(),
            None,
            "disk_db/state_commitment/prune",
        );

        T::prune(&mut buffer, up_to_version)?;

        let (_, pending) = buffer.disassemble();
        let mut batch = WriteBatch::default();
        let cf = cf_state_commitment(&self.inner.db);
        for (key, op) in pending {
            if let Op::Insert(value) = op {
                batch.put_cf(&cf, key, value);
            } else {
                batch.delete_cf(&cf, key);
            }
        }

        // Finally, update the oldest available version value.
        let cf = cf_default(&self.inner.db);
        batch.put_cf(&cf, OLDEST_VERSION_KEY, up_to_version.to_le_bytes());

        self.inner.db.write(batch)?;

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LABEL, "operation" => "prune")
                .record(duration.elapsed().as_secs_f64());
        }

        Ok(())
    }
}

impl<T> DiskDb<T> {
    /// Force an immediate compaction of the timestamped column families.
    pub fn compact(&self) {
        #[cfg(feature = "tracing")]
        {
            tracing::info!("Compacting database");
        }

        #[cfg(feature = "metrics")]
        let duration = std::time::Instant::now();

        self.inner.db.compact_range_cf(
            &cf_state_storage(&self.inner.db),
            None::<&[u8]>,
            None::<&[u8]>,
        );

        #[cfg(feature = "ibc")]
        {
            self.inner.db.compact_range_cf(
                &cf_preimages(&self.inner.db),
                None::<&[u8]>,
                None::<&[u8]>,
            );
        }

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LABEL, "operation" => "compact")
                .record(duration.elapsed().as_secs_f64());
        }
    }
}

// ----------------------------- state commitment ------------------------------

#[derive(Clone, Debug)]
pub struct StateCommitment {
    inner: Arc<DiskDbInner>,
}

impl Storage for StateCommitment {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.inner
            .db
            .get_cf(&cf_state_commitment(&self.inner.db), key)
            .unwrap_or_else(|err| {
                panic!("failed to read from state commitment: {err}");
            })
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let opts = new_read_options(None, min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_state_commitment(&self.inner.db), opts, mode)
            .map(|item| {
                let (k, v) = item.unwrap_or_else(|err| {
                    panic!("failed to iterate in state commitment: {err}");
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
        let opts = new_read_options(None, min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_state_commitment(&self.inner.db), opts, mode)
            .map(|item| {
                let (k, _) = item.unwrap_or_else(|err| {
                    panic!("failed to iterate in state commitment: {err}");
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
        let opts = new_read_options(None, min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_state_commitment(&self.inner.db), opts, mode)
            .map(|item| {
                let (_, v) = item.unwrap_or_else(|err| {
                    panic!("failed to iterate in state commitment: {err}");
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

// ------------------------------- state storage -------------------------------

#[derive(Clone, Debug)]
pub struct StateStorage {
    inner: Arc<DiskDbInner>,
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
    ) -> impl Iterator<Item = Record> + 'a {
        let opts = new_read_options(Some(self.version), min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_state_storage(&self.inner.db), opts, mode)
            .map(|item| {
                let (k, v) = item.unwrap_or_else(|err| {
                    panic!("failed to iterate in state storage: {err}");
                });
                (k.to_vec(), v.to_vec())
            });

        #[cfg(feature = "metrics")]
        let iter = iter.with_metrics(DISK_DB_LABEL, [
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

        let opts = new_read_options(Some(self.version), None, None);
        let value = self
            .inner
            .db
            .get_cf_opt(&cf_state_storage(&self.inner.db), key, &opts)
            .unwrap_or_else(|err| {
                panic!("failed to read from state storage: {err}");
            });

        #[cfg(feature = "metrics")]
        {
            metrics::histogram!(DISK_DB_LABEL, "operation" => "read", "comment" => self.comment)
                .record(duration.elapsed().as_secs_f64());
        }

        value
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
            metrics::histogram!(DISK_DB_LABEL, "operation" => "scan", "comment" => self.comment)
                .record(duration.elapsed().as_secs_f64());
        }

        Box::new(iter)
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
            metrics::histogram!(DISK_DB_LABEL, "operation" => "scan_keys", "comment" => self.comment)
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
            metrics::histogram!(DISK_DB_LABEL, "operation" => "scan_values", "comment" => self.comment)
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

// ---------------------------------- helpers ----------------------------------

#[inline]
fn into_iterator_mode(order: Order) -> IteratorMode<'static> {
    match order {
        Order::Ascending => IteratorMode::Start,
        Order::Descending => IteratorMode::End,
    }
}

// TODO: rocksdb tuning? see:
// https://github.com/sei-protocol/sei-db/blob/main/ss/rocksdb/opts.go#L29-L65
// https://github.com/turbofish-org/merk/blob/develop/src/merk/mod.rs#L84-L102
pub fn new_db_options() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts
}

pub fn new_cf_options_with_ts() -> Options {
    let mut opts = Options::default();
    // Must use a timestamp-enabled comparator
    opts.set_comparator_with_ts(
        U64Comparator::NAME,
        U64Timestamp::SIZE,
        Box::new(U64Comparator::compare),
        Box::new(U64Comparator::compare_ts),
        Box::new(U64Comparator::compare_without_ts),
    );
    opts
}

pub(crate) fn new_read_options(
    version: Option<u64>,
    iterate_lower_bound: Option<&[u8]>,
    iterate_upper_bound: Option<&[u8]>,
) -> ReadOptions {
    let mut opts = ReadOptions::default();
    if let Some(version) = version {
        opts.set_timestamp(U64Timestamp::from(version));
    }
    if let Some(bound) = iterate_lower_bound {
        opts.set_iterate_lower_bound(bound);
    }
    if let Some(bound) = iterate_upper_bound {
        opts.set_iterate_upper_bound(bound);
    }
    opts
}

pub fn cf_default(db: &DBWithThreadMode<MultiThreaded>) -> Arc<BoundColumnFamily<'_>> {
    db.cf_handle(CF_NAME_DEFAULT).unwrap_or_else(|| {
        panic!("failed to find default column family");
    })
}

#[cfg(feature = "ibc")]
pub(crate) fn cf_preimages(db: &DBWithThreadMode<MultiThreaded>) -> Arc<BoundColumnFamily<'_>> {
    db.cf_handle(CF_NAME_PREIMAGES).unwrap_or_else(|| {
        panic!("failed to find default column family");
    })
}

pub fn cf_state_storage(db: &DBWithThreadMode<MultiThreaded>) -> Arc<BoundColumnFamily<'_>> {
    db.cf_handle(CF_NAME_STATE_STORAGE).unwrap_or_else(|| {
        panic!("failed to find state storage column family");
    })
}

pub fn cf_state_commitment(db: &DBWithThreadMode<MultiThreaded>) -> Arc<BoundColumnFamily<'_>> {
    db.cf_handle(CF_NAME_STATE_COMMITMENT).unwrap_or_else(|| {
        panic!("failed to find state commitment column family");
    })
}

// ------------------------ tests using JMT commitment -------------------------

#[cfg(test)]
mod tests_jmt {
    use {
        crate::DiskDb,
        grug_app::Db,
        grug_jmt::{MerkleTree, verify_proof},
        grug_types::{
            Batch, Hash256, HashExt, MembershipProof, NonMembershipProof, Op, Order, Proof,
            ProofNode, Storage,
        },
        hex_literal::hex,
        temp_rocksdb::TempDataDir,
    };

    // Using the same test case as in our rust-rocksdb fork:
    // https://github.com/left-curve/rust-rocksdb/blob/v0.21.0-cw/tests/test_timestamp.rs#L150
    //
    // hash(donald)  = 01000001...
    // hash(jake)    = 11001101...
    // hash(joe)     = 01111000...
    // hash(larry)   = 00001101...
    // hash(pumpkin) = 11111111...

    // The tree at version 1 should look like:
    //
    //           root
    //         ┌──┴──┐
    //         0     jake
    //      ┌──┴──┐
    //    larry   01
    //         ┌──┴──┐
    //      donald  joe
    //
    // hash_00
    // = hash(01 | hash("larry") | hash("engineer"))
    // = hash(01 | 0d098b1c0162939e05719f059f0f844ed989472e9e6a53283a00fe92127ac27f | 7826b958b79c70626801b880405eb5111557dadceb2fee2b1ed69a18eed0c6dc)
    // = 01d2b46c3dd0180a5e8236137b4ada8ae6c9ca7c8799ecf7932d1320c9dfbf3b
    //
    // hash_010
    // = hash(01 | hash("donald") | hash("trump"))
    // = hash(01 | 4138cfbc5d36f31e8ae09ef4044bb88c0c9c6f289a6a1c27b335a99d1d8dc86f | a60a52382d7077712def2a69eda3ba309b19598944aa459ce418ae53b7fb5d58)
    // = 8fb3cdb9c15244dc8b7f701bb08640389dcde92a3b85277348ca1ec839d2a575
    //
    // hash_011
    // = hash(01 | hash("joe") | hash("biden"))
    // = hash(01 | 78675cc176081372c43abab3ea9fb70c74381eb02dc6e93fb6d44d161da6eeb3 | 0631a609edb7c79f3a051b935ddb0927818ebd03964a4d18f316d2dadf216894)
    // = hash(3b640fe6cffebfa7c2ba388b66aa3a4978c2221799ef9316e059eed2e656511a)
    //
    // hash_01
    // = hash(00 | 8fb3cdb9c15244dc8b7f701bb08640389dcde92a3b85277348ca1ec839d2a575 | 3b640fe6cffebfa7c2ba388b66aa3a4978c2221799ef9316e059eed2e656511a)
    // = 248f2dfa7cd94e3856e5a6978e500e6d9528837cd0c64187b937455f8d865baf
    //
    // hash_0
    // = hash(00 | 01d2b46c3dd0180a5e8236137b4ada8ae6c9ca7c8799ecf7932d1320c9dfbf3b | 248f2dfa7cd94e3856e5a6978e500e6d9528837cd0c64187b937455f8d865baf)
    // = 4d28a7511b5df59d1cdab1ace2314ba10f4637d0b51cac24ad0dbf199f7333ad
    //
    // hash_1
    // = hash(01 | hash("jake") | hash("shepherd"))
    // = hash(01 | cdf30c6b345276278bedc7bcedd9d5582f5b8e0c1dd858f46ef4ea231f92731d | def3735d7a0d2696775d6d72f379e4536c4d9e3cd6367f27a0bcb7f40d4558fb)
    // = 8358fe5d68c2d969c72b67ccffef68e2bf3b2edb200c0a7731e9bf131be11394
    //
    // root_hash
    // = hash(00 | 4d28a7511b5df59d1cdab1ace2314ba10f4637d0b51cac24ad0dbf199f7333ad | 8358fe5d68c2d969c72b67ccffef68e2bf3b2edb200c0a7731e9bf131be11394)
    // = 1712a8d4c9896a8cadb4e13592bd9e2713a16d0bf5572a8bf540eb568cb30b64
    mod v0 {
        use super::*;

        pub const ROOT_HASH: Hash256 = Hash256::from_inner(hex!(
            "1712a8d4c9896a8cadb4e13592bd9e2713a16d0bf5572a8bf540eb568cb30b64"
        ));
        pub const HASH_0: Hash256 = Hash256::from_inner(hex!(
            "4d28a7511b5df59d1cdab1ace2314ba10f4637d0b51cac24ad0dbf199f7333ad"
        ));
        pub const HASH_00: Hash256 = Hash256::from_inner(hex!(
            "01d2b46c3dd0180a5e8236137b4ada8ae6c9ca7c8799ecf7932d1320c9dfbf3b"
        ));
        pub const HASH_01: Hash256 = Hash256::from_inner(hex!(
            "248f2dfa7cd94e3856e5a6978e500e6d9528837cd0c64187b937455f8d865baf"
        ));
        pub const HASH_010: Hash256 = Hash256::from_inner(hex!(
            "8fb3cdb9c15244dc8b7f701bb08640389dcde92a3b85277348ca1ec839d2a575"
        ));
        pub const HASH_011: Hash256 = Hash256::from_inner(hex!(
            "3b640fe6cffebfa7c2ba388b66aa3a4978c2221799ef9316e059eed2e656511a"
        ));
        pub const HASH_1: Hash256 = Hash256::from_inner(hex!(
            "8358fe5d68c2d969c72b67ccffef68e2bf3b2edb200c0a7731e9bf131be11394"
        ));
    }

    // The tree at version 2 should look like:
    //
    //            root
    //         ┌───┴───┐
    //         0       1
    //      ┌──┴──┐    └──┐
    //   larry  donald    11
    //                ┌───┴───┐
    //              jake   pumpkin
    //
    // hash_00
    // = 01d2b46c3dd0180a5e8236137b4ada8ae6c9ca7c8799ecf7932d1320c9dfbf3b (same as v1)
    //
    // hash_01
    // = hash(01 | hash("donald") | hash("duck"))
    // = hash(01 | 4138cfbc5d36f31e8ae09ef4044bb88c0c9c6f289a6a1c27b335a99d1d8dc86f | 2d2370db2447ff8cf4f3accd68c85aa119a9c893effd200a9b69176e9fc5eb98)
    // = 44cb87f51dbe89d482329a5cc71fadf6758d3c3f7a46b8e03efbc9354e4b5be7
    //
    // hash_0
    // = hash(00 | 01d2b46c3dd0180a5e8236137b4ada8ae6c9ca7c8799ecf7932d1320c9dfbf3b | 44cb87f51dbe89d482329a5cc71fadf6758d3c3f7a46b8e03efbc9354e4b5be7)
    // = 7ce76869da6e1ff26f873924e6667e131761ef9075aebd6bba7c48663696f402
    //
    // hash_110
    // = 8358fe5d68c2d969c72b67ccffef68e2bf3b2edb200c0a7731e9bf131be11394 (same as in v1)
    //
    // hash_111
    // = hash(01 | hash("pumpkin") | hash("cat"))
    // = hash(01 | ff48e511e1638fc379cb75de1c28fe2016051b167f9aa8cac3dd86c6f4787539 | 77af778b51abd4a3c51c5ddd97204a9c3ae614ebccb75a606c3b6865aed6744e)
    // = a2cb2e0c6a5b3717d5355d1e8d046f305f7bd9730cf94434b51063209664f9c6
    //
    // hash_11
    // = hash(00 | 8358fe5d68c2d969c72b67ccffef68e2bf3b2edb200c0a7731e9bf131be11394 | a2cb2e0c6a5b3717d5355d1e8d046f305f7bd9730cf94434b51063209664f9c6)
    // = 1fd4c7d63c6349b827d1af289d9870f923d0be6ecbb6b91c2f42d81ac7b45a51
    //
    // hash_1
    // = hash(00 | 0000000000000000000000000000000000000000000000000000000000000000 | 1fd4c7d63c6349b827d1af289d9870f923d0be6ecbb6b91c2f42d81ac7b45a51)
    // = 9445f09716426120318220f103d9925c8a73155cf561ed4440b3d1fdc1f1153f
    //
    // root_hash
    // = hash(00 | 7ce76869da6e1ff26f873924e6667e131761ef9075aebd6bba7c48663696f402 | 9445f09716426120318220f103d9925c8a73155cf561ed4440b3d1fdc1f1153f)
    // = 05c5d1c5e433ed85c4b5c42d4da7adf6d204d3c1af37cac316f47b042c154eb4
    mod v1 {
        use super::*;

        pub const ROOT_HASH: Hash256 = Hash256::from_inner(hex!(
            "05c5d1c5e433ed85c4b5c42d4da7adf6d204d3c1af37cac316f47b042c154eb4"
        ));
        pub const HASH_0: Hash256 = Hash256::from_inner(hex!(
            "7ce76869da6e1ff26f873924e6667e131761ef9075aebd6bba7c48663696f402"
        ));
        pub const HASH_00: Hash256 = Hash256::from_inner(hex!(
            "01d2b46c3dd0180a5e8236137b4ada8ae6c9ca7c8799ecf7932d1320c9dfbf3b"
        ));
        pub const HASH_01: Hash256 = Hash256::from_inner(hex!(
            "44cb87f51dbe89d482329a5cc71fadf6758d3c3f7a46b8e03efbc9354e4b5be7"
        ));
        pub const HASH_1: Hash256 = Hash256::from_inner(hex!(
            "9445f09716426120318220f103d9925c8a73155cf561ed4440b3d1fdc1f1153f"
        ));
        pub const HASH_110: Hash256 = Hash256::from_inner(hex!(
            "8358fe5d68c2d969c72b67ccffef68e2bf3b2edb200c0a7731e9bf131be11394"
        ));
        pub const HASH_111: Hash256 = Hash256::from_inner(hex!(
            "a2cb2e0c6a5b3717d5355d1e8d046f305f7bd9730cf94434b51063209664f9c6"
        ));
    }

    #[test]
    fn disk_db_works() {
        let path = TempDataDir::new("_grug_disk_db_works");
        let db = DiskDb::<MerkleTree>::open(&path).unwrap();

        // Write a batch. The very first batch have version 0.
        let batch = Batch::from([
            (b"donald".to_vec(), Op::Insert(b"trump".to_vec())),
            (b"jake".to_vec(), Op::Insert(b"shepherd".to_vec())),
            (b"joe".to_vec(), Op::Insert(b"biden".to_vec())),
            (b"larry".to_vec(), Op::Insert(b"engineer".to_vec())),
        ]);
        let (version, root_hash) = db.flush_and_commit(batch).unwrap();
        assert_eq!(version, 0);
        assert_eq!(root_hash, Some(v0::ROOT_HASH));

        // Write another batch with version = 1.
        let batch = Batch::from([
            (b"donald".to_vec(), Op::Insert(b"duck".to_vec())),
            (b"joe".to_vec(), Op::Delete),
            (b"pumpkin".to_vec(), Op::Insert(b"cat".to_vec())),
        ]);
        let (version, root_hash) = db.flush_and_commit(batch).unwrap();
        assert_eq!(version, 1);
        assert_eq!(root_hash, Some(v1::ROOT_HASH));

        // Try query values at the two versions, respectively, from state storage.
        for (version, key, value) in [
            (0, "donald", Some("trump")),
            (0, "jake", Some("shepherd")),
            (0, "joe", Some("biden")),
            (0, "larry", Some("engineer")),
            (0, "pumpkin", None),
            (1, "donald", Some("duck")),
            (1, "jake", Some("shepherd")),
            (1, "joe", None),
            (1, "larry", Some("engineer")),
            (1, "pumpkin", Some("cat")),
        ] {
            let found_value = db
                .state_storage(Some(version))
                .unwrap()
                .read(key.as_bytes())
                .map(|bz| String::from_utf8(bz).unwrap());
            assert_eq!(found_value.as_deref(), value);
        }

        // Try iterating at the two versions, respectively.
        for (version, items) in [
            (0, [
                ("donald", "trump"),
                ("jake", "shepherd"),
                ("joe", "biden"),
                ("larry", "engineer"),
            ]),
            (1, [
                ("donald", "duck"),
                ("jake", "shepherd"),
                ("larry", "engineer"),
                ("pumpkin", "cat"),
            ]),
        ] {
            for ((found_key, found_value), (key, value)) in db
                .state_storage(Some(version))
                .unwrap()
                .scan(None, None, Order::Ascending)
                .zip(items)
            {
                assert_eq!(found_key, key.as_bytes());
                assert_eq!(found_value, value.as_bytes());
            }
        }

        // Try generating merkle proofs at the two versions, respectively; also
        // verify the proofs.
        for (version, key, value, proof) in [
            (
                0,
                "donald",
                Some("trump"),
                Proof::Membership(MembershipProof {
                    sibling_hashes: vec![Some(v0::HASH_011), Some(v0::HASH_00), Some(v0::HASH_1)],
                }),
            ),
            (
                0,
                "jake",
                Some("shepherd"),
                Proof::Membership(MembershipProof {
                    sibling_hashes: vec![Some(v0::HASH_0)],
                }),
            ),
            (
                0,
                "joe",
                Some("biden"),
                Proof::Membership(MembershipProof {
                    sibling_hashes: vec![Some(v0::HASH_010), Some(v0::HASH_00), Some(v0::HASH_1)],
                }),
            ),
            (
                0,
                "larry",
                Some("engineer"),
                Proof::Membership(MembershipProof {
                    sibling_hashes: vec![Some(v0::HASH_01), Some(v0::HASH_1)],
                }),
            ),
            (
                0,
                "pumpkin",
                None,
                Proof::NonMembership(NonMembershipProof {
                    node: ProofNode::Leaf {
                        key_hash: "jake".hash256(),
                        value_hash: "shepherd".hash256(),
                    },
                    sibling_hashes: vec![Some(v0::HASH_0)],
                }),
            ),
            (
                1,
                "donald",
                Some("duck"),
                Proof::Membership(MembershipProof {
                    sibling_hashes: vec![Some(v1::HASH_00), Some(v1::HASH_1)],
                }),
            ),
            (
                1,
                "jake",
                Some("shepherd"),
                Proof::Membership(MembershipProof {
                    sibling_hashes: vec![Some(v1::HASH_111), None, Some(v1::HASH_0)],
                }),
            ),
            (
                1,
                "joe",
                None,
                Proof::NonMembership(NonMembershipProof {
                    node: ProofNode::Leaf {
                        key_hash: "donald".hash256(),
                        value_hash: "duck".hash256(),
                    },
                    sibling_hashes: vec![Some(v1::HASH_00), Some(v1::HASH_1)],
                }),
            ),
            (
                1,
                "larry",
                Some("engineer"),
                Proof::Membership(MembershipProof {
                    sibling_hashes: vec![Some(v1::HASH_01), Some(v1::HASH_1)],
                }),
            ),
            (
                1,
                "pumpkin",
                Some("cat"),
                Proof::Membership(MembershipProof {
                    sibling_hashes: vec![Some(v1::HASH_110), None, Some(v1::HASH_0)],
                }),
            ),
        ] {
            let found_proof = db.prove(key.as_bytes(), Some(version)).unwrap();
            assert_eq!(found_proof, proof);

            let root_hash = match version {
                0 => v0::ROOT_HASH,
                1 => v1::ROOT_HASH,
                _ => unreachable!(),
            };
            assert!(
                verify_proof(
                    root_hash,
                    key.as_bytes().hash256(),
                    value.map(|v| v.hash256()),
                    &found_proof,
                )
                .is_ok()
            );
        }
    }

    #[test]
    fn disk_db_pruning_works() {
        let path = TempDataDir::new("_grug_disk_db_pruning_works");
        let db = DiskDb::<MerkleTree>::open(&path).unwrap();

        // Apply a few batches. Same test data as used in the JMT test.
        for batch in [
            // v0
            Batch::from([
                (b"r".to_vec(), Op::Insert(b"foo".to_vec())),
                (b"m".to_vec(), Op::Insert(b"bar".to_vec())),
                (b"L".to_vec(), Op::Insert(b"fuzz".to_vec())),
                (b"a".to_vec(), Op::Insert(b"buzz".to_vec())),
            ]),
            // v1
            Batch::from([(b"m".to_vec(), Op::Delete)]),
            // v2
            Batch::from([(b"r".to_vec(), Op::Delete)]),
            // v3
            Batch::from([(b"L".to_vec(), Op::Delete)]),
            // v4
            Batch::from([(b"a".to_vec(), Op::Delete)]),
        ] {
            db.flush_and_commit(batch).unwrap();
        }

        // Prune up to v3.
        // This deletes version 0-2. v3 is now the oldest available version.
        db.prune(3).unwrap();

        // Attempt access the state under versions 0..=2, should fail.
        for version in 0..=2 {
            // Request state storage. Should fail with `DbError::VersionTooOld`.
            assert!(db.state_storage(Some(version)).is_err_and(|err| {
                err.to_string()
                    .contains("older than the oldest available version (3)")
            }));

            // Prove a key. Should fail when attempting to load the root node of
            // that version.
            assert!(db.prove(b"a", Some(version)).is_err_and(|err| {
                err.to_string()
                    .contains("data not found! type: grug_jmt::node::Node")
            }));
        }

        // Doing the same under versions 3, which haven't been pruned, should work.
        // Proof doesn't work for version 4 though, because the tree is empty.
        // We can't proof anything if the tree is empty...
        {
            assert!(db.state_storage(Some(3)).is_ok());
            assert!(db.prove(b"a", Some(3)).is_ok());
        }

        // Doing the same under version 5 (newer than the latest version) should fail.
        {
            assert!(db.state_storage(Some(5)).is_err_and(|err| {
                err.to_string()
                    .contains("newer than the latest version (4)")
            }));

            assert!(db.prove(b"a", Some(5)).is_err_and(|err| {
                err.to_string()
                    .contains("data not found! type: grug_jmt::node::Node")
            }));
        }
    }
}

// ----------------------- tests using simple commitment -----------------------

#[cfg(test)]
mod tests_simple {
    use {
        super::*,
        grug_app::SimpleCommitment,
        grug_types::{ResultExt, btree_map, hash},
        temp_rocksdb::TempDataDir,
    };

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
        let db = DiskDb::<SimpleCommitment>::open(&path).unwrap();

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
        let db = DiskDb::<SimpleCommitment>::open(&path).unwrap();

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

        db.flush_and_commit(Batch::from([(b"2".to_vec(), Op::Insert(b"22".to_vec()))]))
            .unwrap();

        assert_eq!(
            db.state_storage(Some(5)).unwrap().read(b"2"),
            Some(b"22".to_vec())
        );

        assert_eq!(
            db.state_storage(None).unwrap().read(b"2"),
            Some(b"22".to_vec())
        );

        // Prune the db at 3, which is exclusive (3 becomes the oldest version)
        // and force an immediate compaction.
        db.prune(3).unwrap();
        db.compact();

        assert_eq!(
            db.state_storage(Some(4)).unwrap().read(b"2"),
            Some(b"2".to_vec())
        );

        assert_eq!(
            db.state_storage(Some(3)).unwrap().read(b"2"),
            Some(b"2".to_vec())
        );

        // Try to read at version = 2. We return an error trying create a storage at a pruned version.
        // This is not ensuring that we really pruned the db, but that we saved the correct oldest version.
        db.state_storage(Some(2)).should_fail_with_error(
            "requested version (2) is older than the oldest available version (3)",
        );

        // Ensure that the prune really pruned the db.
        db.inner
            .db
            .get_cf_opt(
                &cf_state_storage(&db.inner.db),
                b"2",
                &new_read_options(Some(2), None, None),
            )
            .should_fail_with_error(
                "Invalid argument: Read timestamp:  is smaller than full_history_ts_low",
            );
    }

    #[test]
    fn auto_prune_works() {
        const KEY: &[u8] = b"data";

        let path = TempDataDir::new("_grug_disk_db_auto_prune_works");
        let db = DiskDb::<SimpleCommitment>::open_with_cfg(&path, Config {
            prune_interval: 2,
            prune_keep_recent: 2,
            prune_force_compact: true,
        })
        .unwrap();

        // Write four batches of versions 0, 1, 2, 3.
        for version in 0..=3 {
            db.flush_and_commit(btree_map! {
                KEY.to_vec() => Op::Insert(vec![version]),
            })
            .unwrap();
        }

        // No pruning should have happened at this point. Oldest version should
        // not have been set (it's only set the first time a pruning is performed),
        // and we should be able to read the historical values.
        assert_eq!(db.oldest_version(), None);
        for version in 0..=3 {
            assert_eq!(
                db.state_storage(Some(version)).unwrap().read(KEY).unwrap(),
                vec![version as u8]
            );
        }

        // Write a batch of version 4.
        db.flush_and_commit(btree_map! {
            KEY.to_vec() => Op::Insert(vec![4]),
        })
        .unwrap();

        // A pruning should have happened. with `up_to_version` = 4 - 2 = 2
        // (exclusive, meaning version 0 & 1 are gone, version 2 is kept).
        assert_eq!(db.oldest_version(), Some(2));
        for version in 0..=1 {
            db.state_storage(Some(version))
                .should_fail_with_error(DbError::version_too_old(version, 2));

            db.inner
                .db
                .get_cf_opt(
                    &cf_state_storage(&db.inner.db),
                    KEY,
                    &new_read_options(Some(version), None, None),
                )
                .should_fail_with_error(
                    "Invalid argument: Read timestamp:  is smaller than full_history_ts_low",
                );
        }
        for version in 2..=4 {
            assert_eq!(
                db.state_storage(Some(version)).unwrap().read(KEY).unwrap(),
                vec![version as u8]
            );
        }
    }
}
