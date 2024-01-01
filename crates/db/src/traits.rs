use {
    crate::Batch,
    cw_std::{Order, Record},
    std::ops::Bound,
};

// mirrors cw_std::Storage trait. this is what the storage module must implement
// on the host side.
pub trait Storage {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>>;

    /// If order is ascending, return the _smallest_ key that is in the bound
    /// [min, max) and the associated record;
    /// if order is descending, return the _biggest_ key that is in the bound
    /// [min, max) and the associated record.
    ///
    /// Unlike cw_std::Storage, we can't have a `scan` method that returns an
    /// iterator, because we can't pass an iterator over the Rust<>Wasm FFI.
    fn range_next(
        &self,
        min:   Bound<&Vec<u8>>,
        max:   Bound<&Vec<u8>>,
        order: Order,
    ) -> anyhow::Result<Option<Record>>;

    /// Collect KV data in the store into a vector. Mainly useful in tests.
    #[cfg(test)]
    fn to_vec(&self, order: Order) -> anyhow::Result<Vec<Record>> {
        let mut records = Vec::<Record>::new();
        loop {
            let last = records.last().map_or(Bound::Unbounded, |(k, _)| Bound::Excluded(k.clone()));
            let (min, max) = match order {
                Order::Ascending => (last, Bound::Unbounded),
                Order::Descending => (Bound::Unbounded, last),
            };
            if let Some(record) = self.range_next(min.as_ref(), max.as_ref(), order)? {
                records.push(record);
            } else {
                break;
            }
        }
        Ok(records)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()>;

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()>;
}

/// A trait describing a database object that can atomically write a batch of
/// puts and deletes.
pub trait Committable: Storage {
    /// Apply a batch of DB ops atomically.
    fn apply(&mut self, batch: Batch) -> anyhow::Result<()>;
}
