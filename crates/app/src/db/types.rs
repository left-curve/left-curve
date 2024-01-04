use {anyhow::anyhow, std::collections::BTreeMap};

/// A batch of Db ops, ready to be committed.
/// For RocksDB, this is similar to rocksdb::WriteBatch.
pub type WriteBatch = BTreeMap<Vec<u8>, Op>;

/// Represents a database operation, either inserting a value or deleting one.
#[derive(Debug, Clone)]
pub enum Op {
    Put(Vec<u8>),
    Delete,
}
