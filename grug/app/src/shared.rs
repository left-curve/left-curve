use {
    grug_types::{extend_one_byte, Batch, Order, Record, Storage},
    std::{
        fmt::Display,
        mem::replace,
        sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
        vec,
    },
};

/// A wrapper over the `Arc<RwLock<T>>` smart pointer, providing some convenience
/// methods.
#[derive(Debug, Default)]
pub struct Shared<S> {
    inner: Arc<RwLock<S>>,
}

impl<S> Shared<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub fn read_access(&self) -> RwLockReadGuard<S> {
        self.inner
            .read()
            .unwrap_or_else(|err| panic!("poisoned lock: {err:?}"))
    }

    pub fn write_access(&self) -> RwLockWriteGuard<S> {
        self.inner
            .write()
            .unwrap_or_else(|err| panic!("poisoned lock: {err:?}"))
    }

    pub fn read_with<F, T>(&self, action: F) -> T
    where
        F: FnOnce(RwLockReadGuard<S>) -> T,
    {
        action(self.read_access())
    }

    pub fn write_with<F, T>(&self, action: F) -> T
    where
        F: FnOnce(RwLockWriteGuard<S>) -> T,
    {
        action(self.write_access())
    }

    /// Return the value inside and replace it with a new one.
    pub fn replace(&self, new_value: S) -> S {
        let mut write = self.write_access();
        replace(&mut write, new_value)
    }

    /// Disassemble the smart pointer and return the inner value.
    ///
    /// Panics if reference count is greater than 1, or if the lock is poisoned.
    pub fn disassemble(self) -> S {
        Arc::try_unwrap(self.inner)
            .unwrap_or_else(|_| panic!("unwrapping Arc when ref count > 1"))
            .into_inner()
            .unwrap_or_else(|err| panic!("poisoned lock: {err:?}"))
    }
}

impl<S> Clone for Shared<S> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// When the inner type is `Storage`, the outer `Shared` also implements `Storage`.
impl<S> Storage for Shared<S>
where
    S: Storage,
{
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.read_access().read(key)
    }

    // This is very tricky! Took me days to figure out how to do `scan` on a
    // shared store.
    //
    // A naive implementation of the `scan` may be something like this:
    //
    // ```rust
    // let storage = self.storage.borrow();
    // storage.scan(min, max, order)
    // ```
    //
    // However, this doesn't work! Compiler would complain:
    //
    // > cannot return value referencing local variable `storage`
    // > returns a value referencing data owned by the current function
    //
    // Basically, `storage` is dropped at the end of the function. The iterator
    // created by `storage.scan()` holds an immutable reference to `storage`, so
    // it cannot be returned.
    //
    // For this reason, we need to collect the records into a Vec and have it
    // owned by the iterator. However we can't collect the entire [min, max)
    // range either, because it can potentially very big.
    //
    // The solution we have for now is to collect 30 records per batch. If the
    // batch reaches the end, we fetch the next batch. 30 is the default page
    // limit we use in contracts, so it's a reasonable value.
    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        Box::new(SharedIter::new(self.read_access(), min, max, order))
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        Box::new(self.scan(min, max, order).map(|(k, _)| k))
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        Box::new(self.scan(min, max, order).map(|(_, v)| v))
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.write_access().write(key, value)
    }

    fn remove(&mut self, key: &[u8]) {
        self.write_access().remove(key)
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        self.write_access().remove_range(min, max)
    }

    fn flush(&mut self, batch: Batch) {
        self.write_access().flush(batch)
    }
}

struct SharedIter<'a, S> {
    storage: RwLockReadGuard<'a, S>,
    batch: vec::IntoIter<Record>,
    min: Option<Vec<u8>>,
    max: Option<Vec<u8>>,
    order: Order,
}

impl<'a, S> SharedIter<'a, S> {
    const BATCH_SIZE: usize = 30;

    pub fn new(
        storage: RwLockReadGuard<'a, S>,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Self {
        Self {
            storage,
            batch: Vec::new().into_iter(),
            min: min.map(|slice| slice.to_vec()),
            max: max.map(|slice| slice.to_vec()),
            order,
        }
    }
}

impl<S> SharedIter<'_, S>
where
    S: Storage,
{
    fn collect_next_batch(&mut self) {
        let batch = self
            .storage
            .scan(self.min.as_deref(), self.max.as_deref(), self.order)
            .take(Self::BATCH_SIZE)
            .collect::<Vec<_>>();

        // Now we need to update the bounds.
        if let Some((key, _)) = batch.iter().last() {
            match self.order {
                Order::Ascending => self.min = Some(extend_one_byte(key.clone())),
                Order::Descending => self.max = Some(key.clone()),
            }
        }

        self.batch = batch.into_iter();
    }
}

impl<S> Iterator for SharedIter<'_, S>
where
    S: Storage,
{
    type Item = Record;

    fn next(&mut self) -> Option<Self::Item> {
        // Get the next record in the batch. If it exists (i.e. the batch hasn't
        // reached end yet) then simply return this record.
        if let Some(record) = self.batch.next() {
            return Some(record);
        }

        // We're here means the batch has reached end. Collect another batch
        // from the store. Return the first record in the new batch (which may
        // be `None`, which means the entire iteration has reached end).
        self.collect_next_batch();
        self.batch.next()
    }
}

impl<S> Display for Shared<S>
where
    S: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.read_access())
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use {super::*, grug_types::MockStorage};

    fn mock_records(min: u32, max: u32, order: Order) -> Vec<Record> {
        let mut records = vec![];
        for i in min..max {
            records.push((i.to_be_bytes().to_vec(), i.to_be_bytes().to_vec()));
        }
        if order == Order::Descending {
            records.reverse();
        }
        records
    }

    #[test]
    fn iterator_works() {
        let mut storage = Shared::new(MockStorage::new());

        for (k, v) in mock_records(1, 100, Order::Ascending) {
            storage.write(&k, &v);
        }

        let lower_bound: u32 = 12;
        let upper_bound: u32 = 89;
        let records = storage
            .scan(
                Some(&lower_bound.to_be_bytes()),
                Some(&upper_bound.to_be_bytes()),
                Order::Ascending,
            )
            .collect::<Vec<_>>();
        assert_eq!(
            records,
            mock_records(lower_bound, upper_bound, Order::Ascending)
        );

        let records = storage
            .scan(None, None, Order::Descending)
            .collect::<Vec<_>>();
        assert_eq!(records, mock_records(1, 100, Order::Descending));
    }

    /// An edge case discovered by @Rhaki.
    /// Our previous implementation contains a bug that fails this case.
    /// See: <https://github.com/left-curve/grug/pull/68>
    #[test]
    fn iterator_edge_case() {
        let mut storage = Shared::new(MockStorage::new());

        // Prepare test data.
        // The data is the number 1 to 100, as strings, sorted as strings.
        // (Meaning, it goes like: 1, 10, 11, 12, ... 19, 2, 20, 21, ...)
        let mut data = (1..=100).map(|x| x.to_string()).collect::<Vec<_>>();
        data.sort();

        // Write the data to storage.
        // We only care about the keys here, ignore the values.
        for x in &data {
            storage.write(x.as_bytes(), &[]);
        }

        // Read data from storage. Should match the original.
        let data2 = storage
            .scan_keys(None, None, Order::Ascending)
            .map(|bytes| String::from_utf8(bytes).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(data, data2);
    }

    #[test]
    fn read_and_write() {
        let old_value = vec![1, 2, 3, 4, 5];
        let shared = Shared::new(old_value.clone());

        assert!(shared.read_with(|inner| *inner == old_value));

        let new_value = vec![6, 7, 8, 9, 10];
        let return_value = shared.replace(new_value.clone());

        assert!(shared.read_with(|inner| *inner == new_value));
        assert_eq!(return_value, old_value);
    }
}
