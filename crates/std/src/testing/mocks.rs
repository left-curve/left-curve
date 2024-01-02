use {
    crate::{Batch, Committable, Op, Order, Record, Storage},
    std::{collections::BTreeMap, iter, ops::Bound},
};

/// An in-memory KV store for testing purpose.
#[derive(Default, Debug, Clone)]
pub struct MockStorage {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MockStorage {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }

    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<Box<dyn Iterator<Item = Record> + 'a>> {
        // BTreeMap::range panics if
        // 1. start > end, or
        // 2. start == end and both are exclusive
        // for us, since we interpret min as inclusive and max as exclusive,
        // only the 1st case apply. however, we don't want to panic, we just
        // return an empty iterator.
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return Ok(Box::new(iter::empty()));
            }
        }

        let min = min.map_or(Bound::Unbounded, |bytes| Bound::Included(bytes.to_vec()));
        let max = max.map_or(Bound::Unbounded, |bytes| Bound::Excluded(bytes.to_vec()));
        let iter = self.data.range((min, max)).map(|(k, v)| (k.clone(), v.clone()));

        if order == Order::Ascending {
            Ok(Box::new(iter))
        } else {
            Ok(Box::new(iter.rev()))
        }
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.data.remove(key);
        Ok(())
    }
}

impl Committable for MockStorage {
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
