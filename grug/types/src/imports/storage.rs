use {
    crate::{Batch, Op, Order, Record},
    dyn_clone::DynClone,
};

/// Describing a KV store that supports read, write, and iteration.
///
/// Note that the store must be clone-able, which is required by Wasmer runtime.
/// We can't use the std library Clone trait, which is not object-safe.
/// We use [DynClone](https://crates.io/crates/dyn-clone) instead, which is
/// object-safe, and use the `clone_trait_object!` macro below to derive std
/// Clone trait for any type that implements Storage.
///
/// The object must also be Send and Sync, which is required by Wasmer runtime.
pub trait Storage: DynClone + Send + Sync {
    /// Read a single key-value pair from the storage.
    ///
    /// Return `None` if the key doesn't exist.
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    /// Iterate over data in the KV store under the given bounds and order.
    ///
    /// Minimum bound is inclusive, maximum bound is exclusive.
    /// If `min` > `max`, an empty iterator is to be returned.
    ///
    /// Note: This is different from the behavior of Rust's `BTreeMap`, which
    /// panics if `min` > `max`.
    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a>;

    /// Similar to `scan`, but only return the keys.
    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a>;

    /// Similar to `scan`, but only return the values.
    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a>;

    /// Write a single key-value pair to the storage.
    fn write(&mut self, key: &[u8], value: &[u8]);

    /// Delete a single key-value pair from the storage.
    ///
    /// No-op if the key doesn't exist.
    fn remove(&mut self, key: &[u8]);

    /// Delete all key-value pairs whose keys are in the given range.
    ///
    /// Similar to `scan`, `min` is inclusive, while `max` is exclusive.
    /// No-op if `min` > `max`.
    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>);

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
            if let Op::Insert(value) = op {
                self.write(&key, &value);
            } else {
                self.remove(&key);
            }
        }
    }
}

// A boxed `Storage` is also a `Storage`.
//
// We need to use dynamic dispatch (i.e. `&dyn Storage` and `Box<dyn Storage>`)
// very often in Grug, because of the use of recursive in handling submessages.
// Each layer of recursion, the storage is wrapped in a `CachedStore<T>`. If
// using static dispatch, the compiler will go into infinite nesting:
// `CachedStore<CachedStore<CachedStore<...>>>` until it reaches the recursion
// limit (default to 128) and we get a compiler error.
impl Storage for Box<dyn Storage> {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.as_ref().read(key)
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        self.as_ref().scan(min, max, order)
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        self.as_ref().scan_keys(min, max, order)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        self.as_ref().scan_values(min, max, order)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.as_mut().write(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.as_mut().remove(key)
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        self.as_mut().remove_range(min, max)
    }

    fn flush(&mut self, batch: Batch) {
        self.as_mut().flush(batch)
    }
}

#[derive(Clone)]
pub struct StorageWrapper<'a> {
    storage: &'a dyn Storage,
}

impl<'a> StorageWrapper<'a> {
    pub fn new(storage: &'a dyn Storage) -> Self {
        Self { storage }
    }
}

impl Storage for StorageWrapper<'_> {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.storage.read(key)
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        self.storage.scan(min, max, order)
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        self.storage.scan_keys(min, max, order)
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        self.storage.scan_values(min, max, order)
    }

    fn write(&mut self, _key: &[u8], _value: &[u8]) {
        unimplemented!("StorageWrapper is read-only");
    }

    fn remove(&mut self, _key: &[u8]) {
        unimplemented!("StorageWrapper is read-only");
    }

    fn remove_range(&mut self, _min: Option<&[u8]>, _max: Option<&[u8]>) {
        unimplemented!("StorageWrapper is read-only");
    }
}

// derive std Clone trait for any type that implements Storage
dyn_clone::clone_trait_object!(Storage);
