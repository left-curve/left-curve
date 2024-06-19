use {
    crate::{Borsh, Bound, Encoding, MapKey, PathBuf, Prefix},
    grug_types::{Order, StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct Map<'a, K, T, E: Encoding<T> = Borsh> {
    namespace: &'a [u8],
    key: PhantomData<K>,
    data: PhantomData<T>,
    encoding: PhantomData<E>,
}

impl<'a, K, T, E> Map<'a, K, T, E>
where
    E: Encoding<T>,
{
    pub const fn new(namespace: &'a str) -> Self {
        // TODO: add a maximum length for namespace
        // see comments of increment_last_byte function for rationale
        Self {
            namespace: namespace.as_bytes(),
            key: PhantomData,
            data: PhantomData,
            encoding: PhantomData,
        }
    }
}

impl<'a, K, T, E> Map<'a, K, T, E>
where
    K: MapKey,
    E: Encoding<T>,
{
    fn path(&self, key: K) -> PathBuf<T, E> {
        let mut raw_keys = key.raw_keys();
        let last_raw_key = raw_keys.pop();
        PathBuf::new(self.namespace, &raw_keys, last_raw_key.as_ref())
    }

    fn no_prefix(&self) -> Prefix<K, T, E, K> {
        Prefix::new(self.namespace, &[])
    }

    pub fn prefix(&self, prefix: K::Prefix) -> Prefix<K::Suffix, T, E, K::Suffix> {
        Prefix::new(self.namespace, &prefix.raw_keys())
    }

    pub fn is_empty(&self, storage: &dyn Storage) -> bool {
        self.keys_raw(storage, None, None, Order::Ascending)
            .next()
            .is_none()
    }

    pub fn has(&self, storage: &dyn Storage, k: K) -> bool {
        self.path(k).as_path().exists(storage)
    }

    pub fn remove(&self, storage: &mut dyn Storage, k: K) {
        self.path(k).as_path().remove(storage)
    }

    #[allow(clippy::type_complexity)]
    pub fn range_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'b> {
        self.no_prefix().range_raw(storage, min, max, order)
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

    pub fn clear(
        &self,
        storage: &mut dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        limit: Option<usize>,
    ) {
        self.no_prefix().clear(storage, min, max, limit)
    }

    pub fn save(&self, storage: &mut dyn Storage, k: K, data: &T) -> StdResult<()> {
        self.path(k).as_path().save(storage, data)
    }

    pub fn may_load(&self, storage: &dyn Storage, k: K) -> StdResult<Option<T>> {
        self.path(k).as_path().may_load(storage)
    }

    pub fn load(&self, storage: &dyn Storage, k: K) -> StdResult<T> {
        self.path(k).as_path().load(storage)
    }

    #[allow(clippy::type_complexity)]
    pub fn range<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b> {
        self.no_prefix().range(storage, min, max, order)
    }

    pub fn update<A, Err>(
        &self,
        storage: &mut dyn Storage,
        k: K,
        action: A,
    ) -> Result<Option<T>, Err>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, Err>,
        Err: From<StdError>,
    {
        self.path(k).as_path().update(storage, action)
    }
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use {
        crate::{Borsh, Encoding, Map, Proto},
        borsh::{BorshDeserialize, BorshSerialize},
        grug_types::MockStorage,
        prost::Message,
        test_case::test_case,
    };

    #[derive(BorshDeserialize, BorshSerialize, Message, PartialEq)]
    struct Foo {
        #[prost(string, tag = "1")]
        name: String,
        #[prost(string, tag = "2")]
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

    fn default<'a, E: Encoding<Foo>>() -> (MockStorage, Map<'a, u64, Foo, E>) {
        let mut storage = MockStorage::new();
        let map = Map::new("borsh");
        map.save(&mut storage, 1, &Foo::new("name_1", "surname_1"))
            .unwrap();
        map.save(&mut storage, 2, &Foo::new("name_2", "surname_2"))
            .unwrap();
        map.save(&mut storage, 3, &Foo::new("name_3", "surname_3"))
            .unwrap();
        map.save(&mut storage, 4, &Foo::new("name_4", "surname_4"))
            .unwrap();
        map.save(&mut storage, 5, &Foo::new("name_5", "surname_5"))
            .unwrap();
        (storage, map)
    }

    #[test_case(default::<Proto>(); "proto")]
    #[test_case(default::<Borsh>(); "borsh")]
    fn test<E: Encoding<Foo>>((storage, map): (MockStorage, Map<u64, Foo, E>)) {
        let first = map.load(&storage, 1).unwrap();
        assert_eq!(first, Foo::new("name_1", "surname_1"));
    }
}
