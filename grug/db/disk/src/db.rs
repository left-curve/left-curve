use {
    crate::{DbError, DbResult, U64Comparator, U64Timestamp},
    grug_app::{Buffer, Db, PrunableDb},
    grug_jmt::{MerkleTree, Proof},
    grug_types::{Batch, Hash256, HashExt, Op, Order, Record, Storage},
    rocksdb::{
        BoundColumnFamily, DBWithThreadMode, IteratorMode, MultiThreaded, Options, ReadOptions,
        WriteBatch,
    },
    std::{
        path::Path,
        sync::{Arc, RwLock},
    },
};

/// We use three column families (CFs) for storing data.
/// The default family is used for metadata. Currently the only metadata we have
/// is the latest version.
const CF_NAME_DEFAULT: &str = "default";

/// The preimage column family maps key hashes to raw keys. This is necessary
/// for generating ICS-23 compatible Merkle proofs.
const CF_NAME_PREIMAGES: &str = "preimages";

/// The state commitment (SC) family stores Merkle tree nodes, which hold hashed
/// key-value pair data. We use this CF for deriving the Merkle root hash for the
/// state (used in consensus) and generating Merkle proofs (used in light clients).
const CF_NAME_STATE_COMMITMENT: &str = "state_commitment";

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
const CF_NAME_STATE_STORAGE: &str = "state_storage";

/// Storage key for the latest version.
const LATEST_VERSION_KEY: &[u8] = b"latest_version";

/// Storage key for the oldest version.
const OLDEST_VERSION_KEY: &[u8] = b"oldest_version";

/// Jellyfish Merkle tree (JMT) using default namespaces.
pub(crate) const MERKLE_TREE: MerkleTree = MerkleTree::new_default();

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
pub struct DiskDb {
    pub(crate) inner: Arc<DiskDbInner>,
}

pub(crate) struct DiskDbInner {
    pub db: DBWithThreadMode<MultiThreaded>,
    // Data that are ready to be persisted to the physical database.
    // Ideally we want to just use a `rocksdb::WriteBatch` here, but it's not
    // thread-safe.
    pending_data: RwLock<Option<PendingData>>,
}

pub(crate) struct PendingData {
    version: u64,
    state_commitment: Batch,
    state_storage: Batch,
}

impl DiskDb {
    /// Create a DiskDb instance by opening a physical RocksDB instance.
    pub fn open<P>(data_dir: P) -> DbResult<Self>
    where
        P: AsRef<Path>,
    {
        // Note: For default and state commitment CFs, don't enable timestamping;
        // for state storage column family, enable timestamping.
        let db = DBWithThreadMode::open_cf_with_opts(&new_db_options(), data_dir, [
            (CF_NAME_DEFAULT, Options::default()),
            (CF_NAME_PREIMAGES, new_cf_options_with_ts()),
            (CF_NAME_STATE_STORAGE, new_cf_options_with_ts()),
            (CF_NAME_STATE_COMMITMENT, Options::default()),
        ])?;

        Ok(Self {
            inner: Arc::new(DiskDbInner {
                db,
                pending_data: RwLock::new(None),
            }),
        })
    }
}

impl Clone for DiskDb {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Db for DiskDb {
    type Error = DbError;
    type Proof = Proof;
    type StateCommitment = StateCommitment;
    type StateStorage = StateStorage;

    fn state_commitment(&self) -> StateCommitment {
        StateCommitment {
            inner: Arc::clone(&self.inner),
        }
    }

    fn state_storage(&self, version: Option<u64>) -> DbResult<StateStorage> {
        // Read the latest version.
        // If it doesn't exist, this means not even a single batch has been
        // written yet (e.g. during `InitChain`). In this case just use zero.
        let latest_version = self.latest_version().unwrap_or(0);

        // If version is unspecified, use the latest version. Otherwise, make
        // sure it's no newer than the latest version.
        let version = match version {
            Some(version) => {
                if version > latest_version {
                    return Err(DbError::VersionTooNew {
                        version,
                        latest_version,
                    });
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
                return Err(DbError::VersionTooOld {
                    version,
                    oldest_version,
                });
            }
        }

        Ok(StateStorage {
            inner: Arc::clone(&self.inner),
            version,
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

    fn root_hash(&self, version: Option<u64>) -> DbResult<Option<Hash256>> {
        let version = version.unwrap_or_else(|| self.latest_version().unwrap_or(0));
        Ok(MERKLE_TREE.root_hash(&self.state_commitment(), version)?)
    }

    fn prove(&self, key: &[u8], version: Option<u64>) -> DbResult<Proof> {
        let version = version.unwrap_or_else(|| self.latest_version().unwrap_or(0));
        Ok(MERKLE_TREE.prove(&self.state_commitment(), key.hash256(), version)?)
    }

    fn flush_but_not_commit(&self, batch: Batch) -> DbResult<(u64, Option<Hash256>)> {
        // A write batch must not already exist. If it does, it means a batch
        // has been flushed, but not committed, then a next batch is flusehd,
        // which indicates some error in the ABCI app's logic.
        if self.inner.pending_data.read()?.is_some() {
            return Err(DbError::PendingDataAlreadySet);
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
        let mut buffer = Buffer::new(self.state_commitment(), None);
        let root_hash = MERKLE_TREE.apply_raw(&mut buffer, old_version, new_version, &batch)?;
        let (_, pending) = buffer.disassemble();

        *(self.inner.pending_data.write()?) = Some(PendingData {
            version: new_version,
            state_commitment: pending,
            state_storage: batch,
        });

        Ok((new_version, root_hash))
    }

    fn commit(&self) -> DbResult<()> {
        let pending = self
            .inner
            .pending_data
            .write()?
            .take()
            .ok_or(DbError::PendingDataNotSet)?;
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
        let cf = cf_preimages(&self.inner.db);
        for (key, op) in &pending.state_storage {
            if let Op::Insert(_) = op {
                batch.put_cf_with_ts(&cf, key.hash256(), ts, key);
            } else {
                batch.delete_cf_with_ts(&cf, key.hash256(), ts);
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

        Ok(self.inner.db.write(batch)?)
    }
}

impl PrunableDb for DiskDb {
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

    fn prune(&self, up_to_version: u64) -> DbResult<()> {
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
        let cf = cf_preimages(&self.inner.db);
        self.inner.db.increase_full_history_ts_low(&cf, ts)?;

        // Prune state commitment.
        let mut buffer = Buffer::new(self.state_commitment(), None);
        MERKLE_TREE.prune(&mut buffer, up_to_version)?;

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

        Ok(self.inner.db.write(batch)?)
    }
}

// ----------------------------- state commitment ------------------------------

pub struct StateCommitment {
    inner: Arc<DiskDbInner>,
}

impl Clone for StateCommitment {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
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

#[derive(Clone)]
pub struct StateStorage {
    inner: Arc<DiskDbInner>,
    version: u64,
}

impl Storage for StateStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        let opts = new_read_options(Some(self.version), None, None);
        self.inner
            .db
            .get_cf_opt(&cf_state_storage(&self.inner.db), key, &opts)
            .unwrap_or_else(|err| {
                panic!("failed to read from state storage: {err}");
            })
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
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
        Box::new(iter)
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let opts = new_read_options(Some(self.version), min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_state_storage(&self.inner.db), opts, mode)
            .map(|item| {
                let (k, _) = item.unwrap_or_else(|err| {
                    panic!("failed to iterate in state storage: {err}");
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
        let opts = new_read_options(Some(self.version), min, max);
        let mode = into_iterator_mode(order);
        let iter = self
            .inner
            .db
            .iterator_cf_opt(&cf_state_storage(&self.inner.db), opts, mode)
            .map(|item| {
                let (_, v) = item.unwrap_or_else(|err| {
                    panic!("failed to iterate in state storage: {err}");
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
fn new_db_options() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts
}

fn new_cf_options_with_ts() -> Options {
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

fn cf_default(db: &DBWithThreadMode<MultiThreaded>) -> Arc<BoundColumnFamily> {
    db.cf_handle(CF_NAME_DEFAULT).unwrap_or_else(|| {
        panic!("failed to find default column family");
    })
}

pub(crate) fn cf_preimages(db: &DBWithThreadMode<MultiThreaded>) -> Arc<BoundColumnFamily> {
    db.cf_handle(CF_NAME_PREIMAGES).unwrap_or_else(|| {
        panic!("failed to find default column family");
    })
}

fn cf_state_storage(db: &DBWithThreadMode<MultiThreaded>) -> Arc<BoundColumnFamily> {
    db.cf_handle(CF_NAME_STATE_STORAGE).unwrap_or_else(|| {
        panic!("failed to find state storage column family");
    })
}

fn cf_state_commitment(db: &DBWithThreadMode<MultiThreaded>) -> Arc<BoundColumnFamily> {
    db.cf_handle(CF_NAME_STATE_COMMITMENT).unwrap_or_else(|| {
        panic!("failed to find state commitment column family");
    })
}

// ----------------------------------- test ------------------------------------

#[cfg(test)]
mod tests {
    use {
        crate::{DiskDb, TempDataDir},
        grug_app::{Db, PrunableDb},
        grug_jmt::{verify_proof, MembershipProof, NonMembershipProof, Proof, ProofNode},
        grug_types::{Batch, Hash256, HashExt, Op, Order, Storage},
        hex_literal::hex,
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
        let db = DiskDb::open(&path).unwrap();

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
            assert!(verify_proof(
                root_hash,
                key.as_bytes().hash256(),
                value.map(|v| v.hash256()),
                &found_proof,
            )
            .is_ok());
        }
    }

    #[test]
    fn disk_db_pruning_works() {
        let path = TempDataDir::new("_grug_disk_db_pruning_works");
        let db = DiskDb::open(&path).unwrap();

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
