use std::collections::BTreeMap;

pub trait Storage {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;

    fn write(&mut self, key: &[u8], value: &[u8]);

    fn remove(&mut self, key: &[u8]);
}

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
}
