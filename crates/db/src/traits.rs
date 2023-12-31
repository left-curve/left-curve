use {cw_vm::HostState, std::collections::BTreeMap};

/// Represents a database operation, either inserting a value or deleting one.
#[derive(Debug, Clone)]
pub enum Op {
    Put(Vec<u8>),
    Delete,
}

/// A batch of Db ops, ready to be committed.
/// For RocksDB, this is similar to rocksdb::WriteBatch.
pub type Batch = BTreeMap<Vec<u8>, Op>;

/// A trait describing a database object that can atomically write a batch of
/// puts and deletes.
pub trait Committable {
    /// Apply a batch of DB ops atomically.
    fn commit(&mut self, batch: Batch) -> anyhow::Result<()>;
}

// default implementation of Committable for HostState. it is just to loop
// through the ops and call `read` or `write`. it is slow and not atomic.
// for production use, make sure to do a performant and atomic implementation.
impl<S: HostState> Committable for S {
    fn commit(&mut self, batch: Batch) -> anyhow::Result<()> {
        batch.into_iter().try_for_each(|(key, op)| {
            if let Op::Put(value) = op {
                self.write(&key, &value)
            } else {
                self.remove(&key)
            }
        })
    }
}
