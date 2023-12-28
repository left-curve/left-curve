use {
    crate::Storage,
    std::{collections::BTreeMap, ops::Bound},
};

/// An in-memory KV store for testing purpose.
pub type MockStorage = BTreeMap<Vec<u8>, Vec<u8>>;

impl Storage for MockStorage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.get(key).cloned()
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        self.insert(key.to_vec(), value.to_vec());
    }

    fn remove(&mut self, key: &[u8]) {
        self.remove(key);
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let min = min.map_or(Bound::Unbounded, |x| Bound::Included(x.to_vec()));
        let max = max.map_or(Bound::Unbounded, |x| Bound::Excluded(x.to_vec()));
        Box::new(self.range((min, max)).map(|(k, v)| (k.clone(), v.clone())))
    }
}
