use {
    crate::{Order, Record, Storage},
    std::{collections::BTreeMap, ops::Bound},
};

/// An in-memory, mock implementatiion of the [`Storage`](crate::Storage) trait
/// for testing purpose.
#[derive(Default, Debug, Clone)]
pub struct MockStorage {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[macro_export]
macro_rules! range_bounds {
    ($min:ident, $max:ident) => {{
        // `BTreeMap::range` panics if
        // 1. start > end, or
        // 2. start == end and both are exclusive
        // For us, since we interpret min as inclusive and max as exclusive,
        // only the 1st case apply. However, we don't want to panic, we just
        // return an empty iterator.
        if let (Some(min), Some(max)) = ($min, $max) {
            if min > max {
                return Box::new(std::iter::empty());
            }
        }

        // Min is inclusive, max is exclusive.
        let min = $min.map_or(Bound::Unbounded, |bytes| Bound::Included(bytes.to_vec()));
        let max = $max.map_or(Bound::Unbounded, |bytes| Bound::Excluded(bytes.to_vec()));

        (min, max)
    }};
}

impl Storage for MockStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let bounds = range_bounds!(min, max);
        let iter = self.data.range(bounds).map(|(k, v)| (k.clone(), v.clone()));
        match order {
            Order::Ascending => Box::new(iter),
            Order::Descending => Box::new(iter.rev()),
        }
    }

    fn scan_keys<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let bounds = range_bounds!(min, max);
        let iter = self.data.range(bounds).map(|(k, _)| k.clone());
        match order {
            Order::Ascending => Box::new(iter),
            Order::Descending => Box::new(iter.rev()),
        }
    }

    fn scan_values<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let bounds = range_bounds!(min, max);
        let iter = self.data.range(bounds).map(|(_, v)| v.clone());
        match order {
            Order::Ascending => Box::new(iter),
            Order::Descending => Box::new(iter.rev()),
        }
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.data.insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.data.remove(key);
    }

    fn remove_range(&mut self, min: Option<&[u8]>, max: Option<&[u8]>) {
        self.data.retain(|k, _| {
            if let Some(min) = min {
                if k.as_slice() < min {
                    return true;
                }
            }

            if let Some(max) = max {
                if max <= k.as_slice() {
                    return true;
                }
            }

            false
        });
    }
}
