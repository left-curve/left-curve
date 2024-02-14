use {
    crate::{CacheStore, DbResult, U64Comparator, U64Timestamp},
    cw_jmt::{MerkleTree, Proof},
    cw_std::{hash, Batch, Hash, Op, Order, Record, Storage},
    rocksdb::{
        BoundColumnFamily, DBWithThreadMode, IteratorMode, MultiThreaded, Options, ReadOptions,
        WriteBatch,
    },
    std::{cell::OnceCell, path::Path, sync::Arc},
};

/// We use three column families (CFs) for storing data.
/// The default family is used for metadata. Currently the only metadata we have
/// is the latest version.
const CF_NAME_DEFAULT: &str = "default";

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
/// https://github.com/cwsoftware123/rust-rocksdb/tree/v0.21.0-cw
const CF_NAME_STATE_STORAGE: &str = "state_storage";

/// Storage key for the latest version.
const LATEST_VERSION_KEY: &[u8] = b"latest_version";

/// Jellyfish Merkle tree (JMT) using default namespaces.
const MERKLE_TREE: MerkleTree = MerkleTree::new_default();

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
/// https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-040-storage-and-smt-state-commitments.md
/// and later refined in ADR-065:
/// https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-065-store-v2.md.
/// It was first pushed to production use by Sei, marketed as SeiDB:
/// https://blog.sei.io/sei-db-the-numbers/
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
pub struct BaseStore {
    db: Arc<DBWithThreadMode<MultiThreaded>>,
}

impl BaseStore {
    /// Create a BaseStore instance by opening a physical RocksDB instance.
    pub fn open(data_dir: impl AsRef<Path>) -> DbResult<Self> {
        // note: for default and state commitment CFs, don't enable timestamping;
        // for state storage column family, enable timestamping.
        let db = DBWithThreadMode::open_cf_with_opts(
            &new_db_options(),
            data_dir,
            [
                (CF_NAME_DEFAULT, new_cf_options()),
                (CF_NAME_STATE_COMMITMENT, new_cf_options()),
                (CF_NAME_STATE_STORAGE, new_cf_options_with_ts()),
            ],
        )?;

        Ok(Self {
            db: Arc::new(db),
        })
    }

    /// Return a `StateCommitment` object which implements the `Storage` trait,
    /// which can be used by the MerkleTree.
    ///
    /// NOTE:
    /// 1. This is only used internally.
    /// 2. `StateCommitment` is read-only. Attempting to call write/remove/flush
    /// leads to panicking. Wrap it in a `CacheStore` instead.
    fn state_commitment(&self) -> StateCommitment {
        StateCommitment {
            db: Arc::clone(&self.db),
        }
    }

    /// Return a `StateStorage` object at the given version (default to the
    /// latest version if unspecified). It implements the `Storage` trait which
    /// can be used by the Wasm host to execute contracts.
    ///
    /// NOTE: `StateStorage` is read-only. Attempting to call write/remove/flush
    /// leads to panicking. Wrap it in a `CacheStore` instead.
    pub fn state_storage(&self, version: Option<u64>) -> StateStorage {
        StateStorage {
            db: Arc::clone(&self.db),
            opts: OnceCell::new(),
            version: version.unwrap_or_else(|| self.latest_version()),
        }
    }

    /// Get the latest version of the state that the database stores.
    pub fn latest_version(&self) -> u64 {
        let maybe_bytes = self.db.get_cf(&cf_default(&self.db), LATEST_VERSION_KEY).unwrap_or_else(|err| {
            panic!("failed to read from default column family: {err}");
        });

        let Some(bytes) = maybe_bytes else {
            return 0;
        };

        let array = bytes.try_into().unwrap_or_else(|bytes: Vec<u8>| {
            panic!("latest version is of incorrect byte length: {}", bytes.len());
        });

        u64::from_le_bytes(array)
    }

    /// Generate Merkle proof for a key at the given version (default to latest
    /// version if not provided).
    pub fn prove(&self, key: &[u8], version: Option<u64>) -> DbResult<Proof> {
        let version = version.unwrap_or_else(|| self.latest_version());
        Ok(MERKLE_TREE.prove(&self.state_commitment(), &hash(key), version)?)
    }

    /// Flush a batch of ops (inserts/deletes) into the database, incrementing
    /// the version. Return the updated version and hash.
    pub fn flush(&self, batch: &Batch) -> DbResult<(u64, Option<Hash>)> {
        let old_version = self.latest_version();
        let new_version = old_version + 1;
        let ts = U64Timestamp::from(new_version);
        let mut write_batch = WriteBatch::default();

        // commit hashed KVs to state commitment
        // note: the column family does not have timestamping enabled
        let cf = cf_state_commitment(&self.db);
        let mut cache = CacheStore::new(self.state_commitment(), None);
        let root_hash = MERKLE_TREE.apply_raw(&mut cache, old_version, new_version, batch)?;
        for (key, op) in cache.pending {
            if let Op::Insert(value) = op {
                write_batch.put_cf(&cf, key, value);
            } else {
                write_batch.delete_cf(&cf, key);
            }
        }

        // write raw KVs to state storage
        // note: the column family *does* have timestamping enabled
        let cf = cf_state_storage(&self.db);
        for (key, op) in batch {
            if let Op::Insert(value) = op {
                write_batch.put_cf_with_ts(&cf, key, ts, value);
            } else {
                write_batch.delete_cf_with_ts(&cf, key, ts);
            }
        }

        // write the latest version
        // note: use little endian encoding; the column family does not have
        // timestamping enabled
        write_batch.put_cf(&cf_default(&self.db), LATEST_VERSION_KEY, new_version.to_le_bytes());

        // write the batch to physical DB
        self.db.write(write_batch)?;

        Ok((new_version, root_hash))
    }
}

// ----------------------------- state commitment ------------------------------

pub struct StateCommitment {
    db: Arc<DBWithThreadMode<MultiThreaded>>,
}

impl Clone for StateCommitment {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

impl Storage for StateCommitment {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db.get_cf(&cf_state_commitment(&self.db), key).unwrap_or_else(|err| {
            panic!("failed to read from state commitment: {err}");
        })
    }

    fn scan<'a>(
        &'a self,
        _min: Option<&[u8]>,
        _max: Option<&[u8]>,
        _order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        unimplemented!("this isn't used by the MerkleTree")
    }

    fn write(&mut self, _key: &[u8], _value: &[u8]) {
        unreachable!("write function called on read-only storage");
    }

    fn remove(&mut self, _key: &[u8]) {
        unreachable!("write function called on read-only storage");
    }

    fn flush(&mut self, _batch: Batch) {
        unreachable!("write function called on read-only storage");
    }
}

// ------------------------------- state storage -------------------------------

pub struct StateStorage {
    db: Arc<DBWithThreadMode<MultiThreaded>>,
    opts: OnceCell<ReadOptions>,
    version: u64,
}

impl StateStorage {
    fn read_opts(&self) -> &ReadOptions {
        self.opts.get_or_init(|| new_read_options(Some(self.version), None, None))
    }
}

impl Clone for StateStorage {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            opts: OnceCell::new(),
            version: self.version,
        }
    }
}

impl Storage for StateStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.db.get_cf_opt(&cf_state_storage(&self.db), key, self.read_opts()).unwrap_or_else(|err| {
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
        let mode = match order {
            Order::Ascending => IteratorMode::Start,
            Order::Descending => IteratorMode::End,
        };
        let iter = self.db.iterator_cf_opt(&cf_state_storage(&self.db), opts, mode).map(|item| {
            let (k, v) = item.unwrap_or_else(|err| {
                panic!("failed to iterate in state storage: {err}");
            });
            (k.to_vec(), v.to_vec())
        });
        Box::new(iter)
    }

    fn write(&mut self, _key: &[u8], _value: &[u8]) {
        unreachable!("write function called on read-only storage");
    }

    fn remove(&mut self, _key: &[u8]) {
        unreachable!("write function called on read-only storage");
    }

    fn flush(&mut self, _batch: Batch) {
        unreachable!("write function called on read-only storage");
    }
}

// ---------------------------------- helpers ----------------------------------

fn new_db_options() -> Options {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    opts
}

fn new_cf_options() -> Options {
    let opts = Options::default();
    // TODO: rocksdb tuning? see:
    // https://github.com/sei-protocol/sei-db/blob/main/ss/rocksdb/opts.go#L29-L65
    // https://github.com/turbofish-org/merk/blob/develop/src/merk/mod.rs#L84-L102
    opts
}

fn new_cf_options_with_ts() -> Options {
    let mut opts = new_cf_options();
    // must use a timestamp-enabled comparator
    opts.set_comparator_with_ts(
        U64Comparator::NAME,
        U64Timestamp::SIZE,
        Box::new(U64Comparator::compare),
        Box::new(U64Comparator::compare_ts),
        Box::new(U64Comparator::compare_without_ts),
    );
    opts
}

fn new_read_options(
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
