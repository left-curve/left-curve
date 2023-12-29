use {
    crate::{Order, Storage},
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
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.data.insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.data.remove(key);
    }

    fn scan<'a>(
        &'a self,
        min:   Bound<&[u8]>,
        max:   Bound<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        // BTreeMap::range panics if
        // 1. min > max, or
        // 2. min == max and both are exclusive
        // however in these cases we don't want to panic, we just return an
        // empty iterator.
        match (&min, &max) {
            (Bound::Included(min) | Bound::Excluded(min), Bound::Included(max) | Bound::Excluded(max)) if min > max => {
                return Box::new(iter::empty());
            },
            (Bound::Excluded(min), Bound::Excluded(max)) if min == max => {
                return Box::new(iter::empty());
            },
            _ => {},
        }

        let min = bound_to_vec(min);
        let max = bound_to_vec(max);
        let iter = self.data.range((min, max)).map(|(k, v)| (k.clone(), v.clone()));

        if order == Order::Ascending {
            Box::new(iter)
        } else {
            Box::new(iter.rev())
        }
    }
}

// TODO: replace with bound.map once stablized (seems like happening soon):
// https://github.com/rust-lang/rust/issues/86026
fn bound_to_vec(bound: Bound<&[u8]>) -> Bound<Vec<u8>> {
    match bound {
        Bound::Included(slice) => Bound::Included(slice.to_vec()),
        Bound::Excluded(slice) => Bound::Excluded(slice.to_vec()),
        Bound::Unbounded => Bound::Unbounded,
    }
}
