use {
    crate::{MapKey, Order, Storage, PathBuf, Prefix},
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{marker::PhantomData, ops::Bound},
};

pub struct Map<K, T> {
    namespace:  &'static [u8],
    _key_type:  PhantomData<K>,
    _data_type: PhantomData<T>,
}

impl<K, T> Map<K, T> {
    pub const fn new(namespace: &'static str) -> Self {
        // TODO: add a maximum length for namespace
        // see comments of increment_last_byte function for rationale
        Self {
            namespace:  namespace.as_bytes(),
            _key_type:  PhantomData,
            _data_type: PhantomData,
        }
    }
}

impl<K, T> Map<K, T>
where
    K: MapKey,
{
    fn path(&self, key: K) -> PathBuf<T> {
        let mut raw_keys = key.raw_keys();
        let last_raw_key = raw_keys.pop();
        PathBuf::new(self.namespace, &raw_keys, last_raw_key.as_ref())
    }

    fn no_prefix(&self) -> Prefix<K, T> {
        Prefix::new(self.namespace, &[])
    }

    pub fn prefix(&self, prefix: K::Prefix) -> Prefix<K::Suffix, T> {
        Prefix::new(self.namespace, &prefix.raw_keys())
    }
}

impl<K, T> Map<K, T>
where
    K: MapKey,
    T: Serialize + DeserializeOwned,
{
    pub fn is_empty(&self, store: &dyn Storage) -> bool {
        self.range(store, Bound::Unbounded, Bound::Unbounded, Order::Ascending)
            .next()
            .is_none()
    }

    pub fn has(&self, store: &dyn Storage, k: K) -> bool {
        self.path(k).as_path().exists(store)
    }

    pub fn may_load(&self, store: &dyn Storage, k: K) -> anyhow::Result<Option<T>> {
        self.path(k).as_path().may_load(store)
    }

    pub fn load(&self, store: &dyn Storage, k: K) -> anyhow::Result<T> {
        self.path(k).as_path().load(store)
    }

    pub fn update<A>(&self, store: &mut dyn Storage, k: K, action: A) -> anyhow::Result<Option<T>>
    where
        A: FnOnce(Option<T>) -> anyhow::Result<Option<T>>
    {
        self.path(k).as_path().update(store, action)
    }

    pub fn save(&self, store: &mut dyn Storage, k: K, data: &T) -> anyhow::Result<()> {
        self.path(k).as_path().save(store, data)
    }

    pub fn remove(&self, store: &mut dyn Storage, k: K) {
        self.path(k).as_path().remove(store)
    }

    pub fn range<'a>(
        &'a self,
        store: &'a dyn Storage,
        min:   Bound<K>,
        max:   Bound<K>,
        order: Order,
    ) -> Box<dyn Iterator<Item = anyhow::Result<(K::Output, T)>> + 'a> {
        self.no_prefix().range(store, min, max, order)
    }

    pub fn clear(&self, store: &mut dyn Storage, limit: Option<usize>) {
        self.no_prefix().clear(store, limit)
    }
}
