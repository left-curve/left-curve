use grug_types::{concat, increment_last_byte, Order, Record, Storage};

#[derive(Clone)]
pub struct PrefixStore {
    storage: Box<dyn Storage>,
    namespace: Vec<u8>,
}

impl PrefixStore {
    pub fn new(storage: Box<dyn Storage>, prefixes: &[&[u8]]) -> Self {
        let mut size = 0;
        for prefix in prefixes {
            size += prefix.len();
        }

        let mut namespace = Vec::with_capacity(size);
        for prefix in prefixes {
            namespace.extend_from_slice(prefix);
        }

        Self { storage, namespace }
    }
}

impl Storage for PrefixStore {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        let prefixed_key = concat(&self.namespace, key);
        self.storage.read(&prefixed_key)
    }

    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let min = match min {
            Some(bytes) => concat(&self.namespace, bytes),
            None => self.namespace.to_vec(),
        };
        let max = match max {
            Some(bytes) => concat(&self.namespace, bytes),
            None => increment_last_byte(self.namespace.to_vec()),
        };
        self.storage.scan(Some(&min), Some(&max), order)
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        let prefixed_key = concat(&self.namespace, key);
        self.storage.write(&prefixed_key, value);
    }

    fn remove(&mut self, key: &[u8]) {
        let prefixed_key = concat(&self.namespace, key);
        self.storage.remove(&prefixed_key);
    }
}
