use {
    crate::{StdError, StdResult},
    dyn_clone::DynClone,
    std::collections::BTreeMap,
};

/// A shorthand for an owned KV pair.
pub type Record = (Vec<u8>, Vec<u8>);

/// A batch of Db ops, ready to be committed.
/// For RocksDB, this is similar to rocksdb::WriteBatch.
pub type Batch<K = Vec<u8>, V = Vec<u8>> = BTreeMap<K, Op<V>>;

/// Represents a database operation, either inserting a value or deleting one.
#[derive(Debug, Clone)]
pub enum Op<V = Vec<u8>> {
    Put(V),
    Delete,
}

impl<V> Op<V> {
    // similar to Option::map
    pub fn map<T>(self, f: fn(V) -> T) -> Op<T> {
        match self {
            Op::Put(v) => Op::Put(f(v)),
            Op::Delete => Op::Delete,
        }
    }
}

/// Describing iteration order.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Order {
    Ascending  = 1,
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
    type Error = StdError;

    fn try_from(value: i32) -> StdResult<Self> {
        match value {
            1 => Ok(Order::Ascending),
            2 => Ok(Order::Descending),
            _ => {
                let reason = format!("must be 1 (asc) or 2 (desc), found {value}");
                Err(StdError::deserialize::<Self>(reason))
            },
        }
    }
}

/// Describing a KV store that supports read, write, and iteration.
///
/// Note that the store must be clone-able, which is required by Wasmer runtime.
/// We can't use the std library Clone trait, which is not object-safe.
/// We use DynClone (https://crates.io/crates/dyn-clone) instead, which is
/// object-safe, and use the `clone_trait_object!` macro below to derive std
/// Clone trait for any type that implements Storage.
pub trait Storage: DynClone {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Iterate over data in the KV store under the given bounds and order.
    /// Minimum bound is inclusive, maximum bound is exclusive.
    /// If min > max, an empty iterator is to be returned.
    ///
    /// NOTE: Rust's BTreeMap panics if max > max. We don't want this behavior.
    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);

    /// Perform a batch of writes and removes altogether, ideally atomically.
    ///
    /// The batch is provided by value instead of by reference (unlike other
    /// trait methods above) because in some implementations a copy/clone can be
    /// avoided this way, improving performance.
    ///
    /// The default implementation here is just looping through the ops and
    /// applying them one by one, which is inefficient and not atomic.
    /// Overwrite this implementation if there are more efficient approaches.
    fn flush(&mut self, batch: Batch) {
        for (key, op) in batch {
            if let Op::Put(value) = op {
                self.write(&key, &value);
            } else {
                self.remove(&key);
            }
        }
    }
}

// derive std Clone trait for any type that implements Storage
dyn_clone::clone_trait_object!(Storage);
