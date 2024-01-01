use std::collections::BTreeMap;

/// Represents a database operation, either inserting a value or deleting one.
#[derive(Debug, Clone)]
pub enum Op {
    Put(Vec<u8>),
    Delete,
}

/// A batch of Db ops, ready to be committed.
/// For RocksDB, this is similar to rocksdb::WriteBatch.
pub type Batch = BTreeMap<Vec<u8>, Op>;
