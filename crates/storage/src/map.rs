use {
    crate::{Borsh, Bound, Codec, PathBuf, Prefix, PrefixBound, Prefixer, PrimaryKey},
    grug_types::{Order, Record, StdError, StdResult, Storage},
    std::{borrow::Cow, marker::PhantomData},
};

pub struct Map<'a, K, T, C = Borsh>
where
    C: Codec<T>,
{
    namespace: &'a [u8],
    key: PhantomData<K>,
    data: PhantomData<T>,
    codec: PhantomData<C>,
}

impl<'a, K, T, C> Map<'a, K, T, C>
where
    C: Codec<T>,
{
    pub const fn new(namespace: &'a str) -> Self {
        // TODO: add a maximum length for namespace
        // see comments of increment_last_byte function for rationale
        Self {
            namespace: namespace.as_bytes(),
            key: PhantomData,
            data: PhantomData,
            codec: PhantomData,
        }
    }
}

impl<'a, K, T, C> Map<'a, K, T, C>
where
    K: PrimaryKey,
    C: Codec<T>,
{
    fn path_raw(&self, key_raw: &[u8]) -> PathBuf<T, C> {
        PathBuf::new(self.namespace, &[], Some(&Cow::Borrowed(key_raw)))
    }

    #[doc(hidden)]
    pub fn path(&self, key: K) -> PathBuf<T, C> {
        let mut raw_keys = key.raw_keys();
        let last_raw_key = raw_keys.pop();
        PathBuf::new(self.namespace, &raw_keys, last_raw_key.as_ref())
    }

    fn no_prefix(&self) -> Prefix<K, T, C> {
        Prefix::new(self.namespace, &[])
    }

    pub fn prefix(&self, prefix: K::Prefix) -> Prefix<K::Suffix, T, C> {
        Prefix::new(self.namespace, &prefix.raw_prefixes())
    }

    pub fn is_empty(&self, storage: &dyn Storage) -> bool {
        self.no_prefix().is_empty(storage)
    }

    // ---------------------- methods for single entries -----------------------

    pub fn has_raw(&self, storage: &dyn Storage, key_raw: &[u8]) -> bool {
        self.path_raw(key_raw).as_path().exists(storage)
    }

    pub fn has(&self, storage: &dyn Storage, key: K) -> bool {
        self.path(key).as_path().exists(storage)
    }

    pub fn may_load_raw(&self, storage: &dyn Storage, key_raw: &[u8]) -> Option<Vec<u8>> {
        self.path_raw(key_raw).as_path().may_load_raw(storage)
    }

    pub fn may_load(&self, storage: &dyn Storage, key: K) -> StdResult<Option<T>> {
        self.path(key).as_path().may_load(storage)
    }

    pub fn load_raw(&self, storage: &dyn Storage, key_raw: &[u8]) -> StdResult<Vec<u8>> {
        self.path_raw(key_raw).as_path().load_raw(storage)
    }

    pub fn load(&self, storage: &dyn Storage, key: K) -> StdResult<T> {
        self.path(key).as_path().load(storage)
    }

    /// Using this function is not recommended. If the key or data isn't
    /// properly serialized, later when you read the data, it will fail to
    /// deserialize and error.
    ///
    /// We prefix the function name with the word "unsafe" to highlight this.
    pub fn unsafe_save_raw(&self, storage: &mut dyn Storage, key_raw: &[u8], data_raw: &[u8]) {
        self.path_raw(key_raw).as_path().save_raw(storage, data_raw)
    }

    pub fn save(&self, storage: &mut dyn Storage, key: K, data: &T) -> StdResult<()> {
        self.path(key).as_path().save(storage, data)
    }

    pub fn remove_raw(&self, storage: &mut dyn Storage, key_raw: &[u8]) {
        self.path_raw(key_raw).as_path().remove(storage)
    }

    pub fn remove(&self, storage: &mut dyn Storage, key: K) {
        self.path(key).as_path().remove(storage)
    }

    pub fn update<A, Err>(
        &self,
        storage: &mut dyn Storage,
        key: K,
        action: A,
    ) -> Result<Option<T>, Err>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, Err>,
        Err: From<StdError>,
    {
        self.path(key).as_path().update(storage, action)
    }

    // -------------------- iteration methods (full bound) ---------------------

    pub fn range_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'b> {
        self.no_prefix().range_raw(storage, min, max, order)
    }

    pub fn range<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b> {
        self.no_prefix().range(storage, min, max, order)
    }

    pub fn keys_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.no_prefix().keys_raw(storage, min, max, order)
    }

    pub fn keys<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'b> {
        self.no_prefix().keys(storage, min, max, order)
    }

    pub fn values_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.no_prefix().values_raw(storage, min, max, order)
    }

    pub fn values<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<T>> + 'b> {
        self.no_prefix().values(storage, min, max, order)
    }

    pub fn clear(&self, storage: &mut dyn Storage, min: Option<Bound<K>>, max: Option<Bound<K>>) {
        self.no_prefix().clear(storage, min, max)
    }

    // ------------------- iteration methods (prefix bound) --------------------

    pub fn prefix_range_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'b> {
        self.no_prefix().prefix_range_raw(storage, min, max, order)
    }

    pub fn prefix_range<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b> {
        self.no_prefix().prefix_range(storage, min, max, order)
    }

    pub fn prefix_keys_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.no_prefix().prefix_keys_raw(storage, min, max, order)
    }

    pub fn prefix_keys<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'b> {
        self.no_prefix().prefix_keys(storage, min, max, order)
    }

    pub fn prefix_values_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.no_prefix().prefix_values_raw(storage, min, max, order)
    }

    pub fn prefix_values<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<T>> + 'b> {
        self.no_prefix().prefix_values(storage, min, max, order)
    }

    pub fn prefix_clear(
        &self,
        storage: &mut dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
    ) {
        self.no_prefix().prefix_clear(storage, min, max)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use {
        crate::{Map, PrefixBound},
        borsh::{BorshDeserialize, BorshSerialize},
        grug_types::{MockStorage, StdResult},
    };

    const FOOS: Map<u64, Foo> = Map::new("foo");

    #[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq, Eq)]
    struct Foo {
        name: String,
        surname: String,
    }

    impl Foo {
        pub fn new(name: &str, surname: &str) -> Self {
            Self {
                name: name.to_string(),
                surname: surname.to_string(),
            }
        }
    }

    fn setup_test() -> MockStorage {
        let mut storage = MockStorage::new();

        for (key, name, surname) in [
            (1, "name_1", "surname_1"),
            (2, "name_2", "surname_2"),
            (3, "name_3", "surname_3"),
            (4, "name_4", "surname_4"),
        ] {
            FOOS.save(&mut storage, key, &Foo::new(name, surname))
                .unwrap();
        }

        storage
    }

    #[test]
    fn map_works() {
        let storage = setup_test();

        let first = FOOS.load(&storage, 1).unwrap();
        assert_eq!(first, Foo::new("name_1", "surname_1"));
    }

    #[test]
    fn range_prefix() {
        const MAP: Map<(u64, &str), String> = Map::new("foo");

        let mut storage = MockStorage::new();

        for (index, addr, desc) in [
            (1, "name_1", "desc_1"),
            (2, "name_2", "desc_2"),
            (2, "name_3", "desc_3"),
            (3, "name_4", "desc_4"),
            (3, "name_5", "desc_5"),
            (4, "name_6", "desc_6"),
        ] {
            MAP.save(&mut storage, (index, addr), &desc.to_string())
                .unwrap();
        }

        // `prefix_range` with a max bound, ascending
        {
            let res = MAP
                .prefix_range(
                    &storage,
                    None,
                    Some(PrefixBound::inclusive(2_u64)),
                    grug_types::Order::Ascending,
                )
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            assert_eq!(res, vec![
                ((1_u64, "name_1".to_string()), "desc_1".to_string()),
                ((2_u64, "name_2".to_string()), "desc_2".to_string()),
                ((2_u64, "name_3".to_string()), "desc_3".to_string()),
            ]);
        }

        // `prefix_range` with a min bound, ascending
        {
            let res = MAP
                .prefix_range(
                    &storage,
                    Some(PrefixBound::exclusive(2_u64)),
                    None,
                    grug_types::Order::Ascending,
                )
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            assert_eq!(res, vec![
                ((3_u64, "name_4".to_string()), "desc_4".to_string()),
                ((3_u64, "name_5".to_string()), "desc_5".to_string()),
                ((4_u64, "name_6".to_string()), "desc_6".to_string()),
            ]);
        }

        // `prefix_range` with a max bound, Descending
        {
            let res = MAP
                .prefix_range(
                    &storage,
                    None,
                    Some(PrefixBound::exclusive(2_u64)),
                    grug_types::Order::Descending,
                )
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            #[rustfmt::skip]
            assert_eq!(res, vec![
                ((1_u64, "name_1".to_string()), "desc_1".to_string()),
            ]);
        }

        // `prefix_range` with a min bound, Descending
        {
            let res = MAP
                .prefix_range(
                    &storage,
                    Some(PrefixBound::inclusive(2_u64)),
                    None,
                    grug_types::Order::Descending,
                )
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            assert_eq!(res, vec![
                ((4_u64, "name_6".to_string()), "desc_6".to_string()),
                ((3_u64, "name_5".to_string()), "desc_5".to_string()),
                ((3_u64, "name_4".to_string()), "desc_4".to_string()),
                ((2_u64, "name_3".to_string()), "desc_3".to_string()),
                ((2_u64, "name_2".to_string()), "desc_2".to_string()),
            ]);
        }

        // `prefix_range` with both min and max bounds, ascending
        {
            let res = MAP
                .prefix_range(
                    &storage,
                    Some(PrefixBound::inclusive(2_u64)),
                    Some(PrefixBound::exclusive(4_u64)),
                    grug_types::Order::Ascending,
                )
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            assert_eq!(res, vec![
                ((2_u64, "name_2".to_string()), "desc_2".to_string()),
                ((2_u64, "name_3".to_string()), "desc_3".to_string()),
                ((3_u64, "name_4".to_string()), "desc_4".to_string()),
                ((3_u64, "name_5".to_string()), "desc_5".to_string()),
            ]);
        }
    }
}
