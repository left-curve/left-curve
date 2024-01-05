use cw_std::storage_utils::trim;

use {
    anyhow::anyhow,
    cw_std::{
        storage_utils::{concat, increment_last_byte},
        Order, Record, Storage,
    },
    cw_vm::Storage as BackendStorage,
    std::collections::HashMap,
};

pub struct PrefixStore<S> {
    store:        S,
    namespace:    Vec<u8>,
    iterators:    HashMap<i32, Iter>,
    next_iter_id: i32,
}

impl<S> PrefixStore<S> {
    pub fn new(store: S, prefixes: &[&[u8]]) -> Self {
        let mut size = 0;
        for prefix in prefixes {
            size += prefix.len();
        }

        let mut namespace = Vec::with_capacity(size);
        for prefix in prefixes {
            namespace.extend_from_slice(prefix);
        }

        Self {
            store,
            namespace,
            iterators:    HashMap::new(),
            next_iter_id: 0,
        }
    }

    pub fn disassemble(self) -> S {
        self.store
    }
}

impl<S: Storage> BackendStorage for PrefixStore<S> {
    fn read(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(self.store.read(&concat(&self.namespace, key)))
    }

    fn scan(
        &mut self,
        min:   Option<&[u8]>,
        max:   Option<&[u8]>,
        order: Order,
    ) -> anyhow::Result<i32> {
        let iterator_id = self.next_iter_id;
        self.next_iter_id += 1;

        let iterator = Iter::new(&self.namespace, min, max, order);
        self.iterators.insert(iterator_id, iterator);

        Ok(iterator_id)
    }

    fn next(&mut self, iterator_id: i32) -> anyhow::Result<Option<Record>> {
        self.iterators
            .get_mut(&iterator_id)
            .map(|iter| iter.next(&self.store))
            .ok_or_else(|| anyhow!("[PrefixStore]: can't find iterator with id {iterator_id}"))
    }

    fn write(&mut self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        self.store.write(&concat(&self.namespace, key), value);

        // whenever KV data is mutated, delete all existing iterators to avoid
        // race conditions.
        self.iterators.clear();

        Ok(())
    }

    fn remove(&mut self, key: &[u8]) -> anyhow::Result<()> {
        self.store.remove(&concat(&self.namespace, key));

        // whenever KV data is mutated, delete all existing iterators to avoid
        // race conditions.
        self.iterators.clear();

        Ok(())
    }
}

struct Iter {
    namespace: Vec<u8>,
    min:       Vec<u8>,
    max:       Vec<u8>,
    order:     Order,
}

impl Iter {
    pub fn new(namespace: &[u8], min: Option<&[u8]>, max: Option<&[u8]>, order: Order) -> Self {
        let min = match min {
            None => namespace.to_vec(),
            Some(min) => concat(namespace, min),
        };
        let max = match max {
            None => increment_last_byte(namespace.to_vec()),
            Some(max) => concat(namespace, max),
        };

        Self {
            namespace: namespace.to_vec(),
            min,
            max,
            order,
        }
    }

    pub fn next(&mut self, store: &dyn Storage) -> Option<Record> {
        let (k, v) = store.scan(Some(&self.min), Some(&self.max), self.order).next()?;

        if self.order == Order::Ascending {
            self.min = increment_last_byte(k.clone());
        } else {
            self.max = k.clone();
        }

        Some((trim(&self.namespace, &k), v))
    }
}
