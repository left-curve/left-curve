use {
    cw_std::{MockStorage, Storage},
    std::collections::BTreeMap,
};

/// A batch of Db ops, ready to be committed.
/// For RocksDB, this is similar to rocksdb::WriteBatch.
pub type Batch = BTreeMap<Vec<u8>, Op>;

/// Represents a database operation, either inserting a value or deleting one.
#[derive(Debug, Clone)]
pub enum Op {
    Put(Vec<u8>),
    Delete,
}

/// Describing a KV store capable to performing a batch of reads/writes together,
/// ideally atomically.
pub trait Flush {
    fn flush(&mut self, batch: Batch) -> anyhow::Result<()>;
}

impl Flush for MockStorage {
    fn flush(&mut self, batch: Batch) -> anyhow::Result<()> {
        for (key, op) in batch {
            if let Op::Put(value) = op {
                self.write(&key, &value);
            } else {
                self.remove(&key);
            }
        }
        Ok(())
    }
}
