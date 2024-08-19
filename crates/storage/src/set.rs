use {
    crate::{Borsh, Bound, Codec, Key, PathBuf, Prefix, PrefixBound, Prefixer},
    grug_types::{Empty, Order, StdResult, Storage},
    std::{borrow::Cow, marker::PhantomData},
};

/// Mimic the behavior of HashSet or BTreeSet.
///
/// Internally, this is basicaly a `Map<T, Empty>`.
///
/// We explicitly use Borsh here, because there's no benefit using any other
/// encoding scheme.
pub struct Set<'a, T, C = Borsh>
where
    C: Codec<Empty>,
{
    pub(crate) namespace: &'a [u8],
    item: PhantomData<T>,
    codec: PhantomData<C>,
}

impl<'a, T, C> Set<'a, T, C>
where
    C: Codec<Empty>,
{
    pub const fn new(namespace: &'a str) -> Self {
        Self {
            namespace: namespace.as_bytes(),
            item: PhantomData,
            codec: PhantomData,
        }
    }
}

impl<'a, T, C> Set<'a, T, C>
where
    T: Key,
    C: Codec<Empty>,
{
    fn path_raw(&self, key_raw: &[u8]) -> PathBuf<Empty, Borsh> {
        PathBuf::new(self.namespace, &[], Some(&Cow::Borrowed(key_raw)))
    }

    fn path(&self, item: T) -> PathBuf<Empty, Borsh> {
        let mut raw_keys = item.raw_keys();
        let last_raw_key = raw_keys.pop();
        PathBuf::new(self.namespace, &raw_keys, last_raw_key.as_ref())
    }

    fn no_prefix(&self) -> Prefix<T, Empty, C> {
        Prefix::new(self.namespace, &[])
    }

    // Return Prefix with I = Empty to allow only keys iterator methods.
    pub fn prefix(&self, prefix: T::Prefix) -> Prefix<T::Suffix, Empty, C, Empty> {
        Prefix::new(self.namespace, &prefix.raw_prefixes())
    }

    pub fn is_empty(&self, storage: &dyn Storage) -> bool {
        self.range_raw(storage, None, None, Order::Ascending)
            .next()
            .is_none()
    }

    // ---------------------- methods for single entries -----------------------

    pub fn has_raw(&self, storage: &dyn Storage, item_raw: &[u8]) -> bool {
        self.path_raw(item_raw).as_path().exists(storage)
    }

    pub fn has(&self, storage: &dyn Storage, item: T) -> bool {
        self.path(item).as_path().exists(storage)
    }

    /// Using this function is not recommended. If the item isn't properly
    /// serialized, later when you read the data, it will fail to deserialize
    /// and error.
    ///
    /// We prefix the function name with the word "unsafe" to highlight this.
    pub fn unsafe_insert_raw(&self, storage: &mut dyn Storage, item_raw: &[u8]) {
        // `Empty` serializes to empty bytes when using borsh.
        self.path_raw(item_raw).as_path().save_raw(storage, &[])
    }

    pub fn insert(&self, storage: &mut dyn Storage, item: T) -> StdResult<()> {
        self.path(item).as_path().save(storage, &Empty {})
    }

    pub fn remove_raw(&self, storage: &mut dyn Storage, item_raw: &[u8]) {
        self.path_raw(item_raw).as_path().remove(storage)
    }

    pub fn remove(&self, storage: &mut dyn Storage, item: T) {
        self.path(item).as_path().remove(storage)
    }

    // -------------------- iteration methods (full bound) ---------------------

    pub fn range_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<T>>,
        max: Option<Bound<T>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.no_prefix().keys_raw(storage, min, max, order)
    }

    pub fn range<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<T>>,
        max: Option<Bound<T>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<T::Output>> + 'b> {
        self.no_prefix().keys(storage, min, max, order)
    }

    pub fn clear(&self, storage: &mut dyn Storage, min: Option<Bound<T>>, max: Option<Bound<T>>) {
        self.no_prefix().clear(storage, min, max)
    }

    // ------------------- iteration methods (prefix bound) --------------------

    pub fn prefix_range_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<PrefixBound<T>>,
        max: Option<PrefixBound<T>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.no_prefix().prefix_keys_raw(storage, min, max, order)
    }

    pub fn prefix_range<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<PrefixBound<T>>,
        max: Option<PrefixBound<T>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<T::Output>> + 'b> {
        self.no_prefix().prefix_keys(storage, min, max, order)
    }

    pub fn prefix_clear(
        &self,
        storage: &mut dyn Storage,
        min: Option<PrefixBound<T>>,
        max: Option<PrefixBound<T>>,
    ) {
        self.no_prefix().prefix_clear(storage, min, max)
    }
}
