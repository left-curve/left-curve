use {anyhow::anyhow, std::collections::BTreeMap};

/// A shorthand for an owned KV pair.
pub type Record = (Vec<u8>, Vec<u8>);

/// A batch of Db ops, ready to be committed.
/// For RocksDB, this is similar to rocksdb::WriteBatch.
pub type Batch = BTreeMap<Vec<u8>, Op>;

/// Represents a database operation, either inserting a value or deleting one.
#[derive(Debug, Clone)]
pub enum Op {
    Put(Vec<u8>),
    Delete,
}

/// Describing iteration order.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Order {
    Ascending = 1,
    Descending = 2,
}

// we need to convert Order into a primitive type such as i32 so that it can be
// passed over FFI
impl From<Order> for i32 {
    fn from(order: Order) -> Self {
        order as _
    }
}

impl TryFrom<i32> for Order {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> anyhow::Result<Self> {
        match value {
            1 => Ok(Order::Ascending),
            2 => Ok(Order::Descending),
            _ => Err(anyhow!("Invalid iterator order {value}, must be 1 (asc) or 2 (desc)")),
        }
    }
}

/// Describing a KV store that supports read, write, and iteration.
pub trait Storage {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    // minimum bound is always inclusive, maximum bound is always exclusive.
    // if min > max, an empty iterator is to be returned.
    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<Box<dyn Iterator<Item = Record> + 'a>>;

    // collect KV data in the store into a vector. useful in tests.
    fn to_vec(&self, order: Order) -> anyhow::Result<Vec<Record>> {
        self.scan(None, None, order).map(|iter| iter.collect())
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()>;

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()>;
}

/// Describing a KV store that can atomically write a batch of ops.
pub trait Committable {
    /// Apply a batch of DB ops atomically.
    fn apply(&mut self, batch: Batch) -> anyhow::Result<()>;
}
