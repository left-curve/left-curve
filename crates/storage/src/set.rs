use {
    crate::{Bound, MapKey, PathBuf, Prefix},
    grug_types::{Empty, Order, StdResult, Storage},
    std::marker::PhantomData,
};

/// Mimic the behavior of HashSet or BTreeSet.
/// Internally, this is basicaly a `Map<T, Empty>`.
pub struct Set<'a, T> {
    namespace: &'a [u8],
    item_type: PhantomData<T>,
}

impl<'a, T> Set<'a, T> {
    pub const fn new(namespace: &'a str) -> Self {
        Self {
            namespace: namespace.as_bytes(),
            item_type: PhantomData,
        }
    }
}

impl<'a, T> Set<'a, T>
where
    T: MapKey,
{
    fn path(&self, item: T) -> PathBuf<Empty> {
        let mut raw_keys = item.raw_keys();
        let last_raw_key = raw_keys.pop();
        PathBuf::new(self.namespace, &raw_keys, last_raw_key.as_ref())
    }

    fn no_prefix(&self) -> Prefix<T, Empty> {
        Prefix::new(self.namespace, &[])
    }

    pub fn prefix(&self, prefix: T::Prefix) -> Prefix<T::Suffix, Empty> {
        Prefix::new(self.namespace, &prefix.raw_keys())
    }

    pub fn is_empty(&self, store: &dyn Storage) -> bool {
        self.range(store, None, None, Order::Ascending).next().is_none()
    }

    pub fn has(&self, store: &dyn Storage, item: T) -> bool {
        self.path(item).as_path().exists(store)
    }

    pub fn insert(&self, store: &mut dyn Storage, item: T) -> StdResult<()> {
        self.path(item).as_path().save(store, &Empty {})
    }

    pub fn remove(&self, store: &mut dyn Storage, item: T) {
        self.path(item).as_path().remove(store)
    }

    pub fn range<'b>(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<T>>,
        max: Option<Bound<T>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<T::Output>> + 'b> {
        self.no_prefix().keys(store, min, max, order)
    }

    pub fn clear(
        &self,
        store: &mut dyn Storage,
        min:   Option<Bound<T>>,
        max:   Option<Bound<T>>,
        limit: Option<usize>,
    ) -> StdResult<()> {
        self.no_prefix().clear(store, min, max, limit)
    }
}
