use std::{
    collections::BTreeMap,
    sync::{LazyLock, Mutex},
};

pub struct TransientMap<K, V>
where
    K: Ord,
{
    inner: LazyLock<Mutex<BTreeMap<K, V>>>,
}

impl<K, V> TransientMap<K, V>
where
    K: Ord,
{
    pub const fn new() -> Self {
        Self {
            inner: LazyLock::new(|| Mutex::new(BTreeMap::new())),
        }
    }

    pub fn insert(&self, key: K, value: V) {
        self.inner.lock().unwrap().insert(key, value);
    }

    pub fn remove(&self, key: &K) -> Option<V> {
        self.inner.lock().unwrap().remove(key)
    }

    pub fn drain(&self) -> BTreeMap<K, V> {
        let mut inner = self.inner.lock().unwrap();
        std::mem::take(&mut *inner)
    }
}
