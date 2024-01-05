use {
    crate::{Order, Record, Storage},
    data_encoding::BASE64,
    std::{collections::BTreeMap, iter, ops::Bound},
    tracing::trace,
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
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        trace!(key = ?BASE64.encode(key), "db_read");
        self.data.get(key).cloned()
    }

    fn scan<'a>(
        &'a self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        trace!(
            min = ?min.map(|bz| BASE64.encode(bz)),
            max = ?max.map(|bz| BASE64.encode(bz)),
            ?order,
            "db_scan",
        );

        // BTreeMap::range panics if
        // 1. start > end, or
        // 2. start == end and both are exclusive
        // for us, since we interpret min as inclusive and max as exclusive,
        // only the 1st case apply. however, we don't want to panic, we just
        // return an empty iterator.
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return Box::new(iter::empty());
            }
        }

        let min = min.map_or(Bound::Unbounded, |bytes| Bound::Included(bytes.to_vec()));
        let max = max.map_or(Bound::Unbounded, |bytes| Bound::Excluded(bytes.to_vec()));
        let iter = self.data.range((min, max)).map(|(k, v)| (k.clone(), v.clone()));

        if order == Order::Ascending {
            Box::new(iter)
        } else {
            Box::new(iter.rev())
        }
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        trace!(key = ?BASE64.encode(key), value = ?BASE64.encode(value), "db_write");
        self.data.insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        trace!(key = ?BASE64.encode(key), "db_remove");
        self.data.remove(key);
    }
}
