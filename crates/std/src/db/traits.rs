use crate::{Batch, Op, Order, Record};

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

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()>;

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()>;

    // collect KV data in the store into a vector. useful in tests.
    fn to_vec(&self, order: Order) -> anyhow::Result<Vec<Record>> {
        self.scan(None, None, order).map(|iter| iter.collect())
    }

    /// Apply a batch of inserts or deletes all together.
    fn apply(&mut self, batch: Batch) -> anyhow::Result<()> {
        for (key, op) in batch {
            if let Op::Put(value) = op {
                self.write(&key, &value)?;
            } else {
                self.remove(&key)?;
            }
        }
        Ok(())
    }
}
