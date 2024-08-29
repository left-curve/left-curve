use {
    crate::{Bound, Codec, PrefixBound, Prefixer, PrimaryKey, RawBound},
    grug_types::{
        concat, encode_length, extend_one_byte, increment_last_byte, nested_namespaces_with_key,
        trim, Order, Record, StdResult, Storage,
    },
    std::{borrow::Cow, marker::PhantomData},
};

pub struct Prefix<K, T, C>
where
    C: Codec<T>,
{
    namespace: Vec<u8>,
    suffix: PhantomData<K>,
    data: PhantomData<T>,
    codec: PhantomData<C>,
}

impl<K, T, C> Prefix<K, T, C>
where
    C: Codec<T>,
{
    pub fn new(namespace: &[u8], prefixes: &[Cow<[u8]>]) -> Self {
        Self {
            namespace: nested_namespaces_with_key(
                Some(namespace),
                prefixes,
                <Option<&Cow<[u8]>>>::None,
            ),
            suffix: PhantomData,
            data: PhantomData,
            codec: PhantomData,
        }
    }
}

impl<K, T, C> Prefix<K, T, C>
where
    K: PrimaryKey,
    C: Codec<T>,
{
    pub fn append(mut self, prefix: K::Prefix) -> Prefix<K::Suffix, T, C> {
        for key_elem in prefix.raw_prefixes() {
            self.namespace.extend(encode_length(&key_elem));
            self.namespace.extend(key_elem.as_ref());
        }

        Prefix {
            namespace: self.namespace,
            suffix: PhantomData,
            data: self.data,
            codec: self.codec,
        }
    }

    pub fn is_empty(&self, storage: &dyn Storage) -> bool {
        self.keys_raw(storage, None, None, Order::Ascending)
            .next()
            .is_none()
    }

    // -------------------- iteration methods (full bound) ---------------------

    pub fn range_raw<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        // Compute start and end bounds.
        // Note that the store considers the start bounds as inclusive, and end
        // bound as exclusive (see the Storage trait).
        let (min, max) = range_bounds(&self.namespace, min, max);

        // Need to make a clone of self.prefix and move it into the closure,
        // so that the iterator can live longer than `&self`.
        let namespace = self.namespace.clone();
        let iter = storage
            .scan(Some(&min), Some(&max), order)
            .map(move |(k, v)| {
                debug_assert_eq!(&k[0..namespace.len()], namespace, "namespace mispatch");
                (trim(&namespace, &k), v)
            });

        Box::new(iter)
    }

    pub fn range<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'a> {
        let iter = self
            .range_raw(storage, min, max, order)
            .map(|(key_raw, value_raw)| {
                let key = K::from_slice(&key_raw)?;
                let value = C::decode(&value_raw)?;
                Ok((key, value))
            });

        Box::new(iter)
    }

    pub fn keys_raw<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let (min, max) = range_bounds(&self.namespace, min, max);
        let namespace = self.namespace.clone();
        let iter = storage
            .scan_keys(Some(&min), Some(&max), order)
            .map(move |k| {
                debug_assert_eq!(&k[0..namespace.len()], namespace, "namespace mispatch");
                trim(&namespace, &k)
            });

        Box::new(iter)
    }

    /// Iterate the raw primary keys under the given index value, without
    /// trimming the prefix (the whole key is returned).
    ///
    /// This is used internally for the indexed map.
    pub(crate) fn keys_raw_no_trimmer<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let (min, max) = range_bounds(&self.namespace, min, max);
        let iter = storage.scan_keys(Some(&min), Some(&max), order);

        Box::new(iter)
    }

    pub fn keys<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'a> {
        let iter = self
            .keys_raw(storage, min, max, order)
            .map(|key_raw| K::from_slice(&key_raw));

        Box::new(iter)
    }

    pub fn values_raw<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let (min, max) = range_bounds(&self.namespace, min, max);
        let iter = storage.scan_values(Some(&min), Some(&max), order);

        Box::new(iter)
    }

    pub fn values<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<T>> + 'a> {
        let iter = self
            .values_raw(storage, min, max, order)
            .map(|value_raw| C::decode(&value_raw));

        Box::new(iter)
    }

    pub fn clear(&self, storage: &mut dyn Storage, min: Option<Bound<K>>, max: Option<Bound<K>>) {
        let (min, max) = range_bounds(&self.namespace, min, max);
        storage.remove_range(Some(&min), Some(&max))
    }

    // ------------------- iteration methods (prefix bound) --------------------

    pub fn prefix_range_raw<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        let (min, max) = range_prefix_bounds(&self.namespace, min, max);
        let namespace = self.namespace.clone();
        let iter = storage
            .scan(Some(&min), Some(&max), order)
            .map(move |(k, v)| {
                debug_assert_eq!(&k[0..namespace.len()], namespace, "namespace mispatch");
                (trim(&namespace, &k), v)
            });

        Box::new(iter)
    }

    pub fn prefix_range<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'a> {
        let iter = self
            .prefix_range_raw(storage, min, max, order)
            .map(|(key_raw, value_raw)| {
                let key = K::from_slice(&key_raw)?;
                let value = C::decode(&value_raw)?;
                Ok((key, value))
            });

        Box::new(iter)
    }

    pub fn prefix_keys_raw<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let (min, max) = range_prefix_bounds(&self.namespace, min, max);
        let namespace = self.namespace.clone();
        let iter = storage
            .scan_keys(Some(&min), Some(&max), order)
            .map(move |k| {
                debug_assert_eq!(&k[0..namespace.len()], namespace, "namespace mispatch");
                trim(&namespace, &k)
            });

        Box::new(iter)
    }

    pub fn prefix_keys<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'a> {
        let iter = self
            .prefix_keys_raw(storage, min, max, order)
            .map(|key_raw| K::from_slice(&key_raw));

        Box::new(iter)
    }

    pub fn prefix_values_raw<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let (min, max) = range_prefix_bounds(&self.namespace, min, max);
        let iter = storage.scan_values(Some(&min), Some(&max), order);

        Box::new(iter)
    }

    pub fn prefix_values<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<T>> + 'a> {
        let iter = self
            .prefix_values_raw(storage, min, max, order)
            .map(|value_raw| C::decode(&value_raw));

        Box::new(iter)
    }

    pub fn prefix_clear(
        &self,
        storage: &mut dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
    ) {
        let (min, max) = range_prefix_bounds(&self.namespace, min, max);
        storage.remove_range(Some(&min), Some(&max))
    }
}

fn range_bounds<K>(
    namespace: &[u8],
    min: Option<Bound<K>>,
    max: Option<Bound<K>>,
) -> (Vec<u8>, Vec<u8>)
where
    K: PrimaryKey,
{
    let min = match min.map(RawBound::from) {
        None => namespace.to_vec(),
        Some(RawBound::Inclusive(k)) => concat(namespace, &k),
        Some(RawBound::Exclusive(k)) => concat(namespace, &extend_one_byte(k)),
    };
    let max = match max.map(RawBound::from) {
        None => increment_last_byte(namespace.to_vec()),
        Some(RawBound::Inclusive(k)) => concat(namespace, &extend_one_byte(k)),
        Some(RawBound::Exclusive(k)) => concat(namespace, &k),
    };

    (min, max)
}

fn range_prefix_bounds<K>(
    namespace: &[u8],
    min: Option<PrefixBound<K>>,
    max: Option<PrefixBound<K>>,
) -> (Vec<u8>, Vec<u8>)
where
    K: PrimaryKey,
{
    let min = match min.map(RawBound::from) {
        None => namespace.to_vec(),
        Some(RawBound::Inclusive(p)) => concat(namespace, &p),
        Some(RawBound::Exclusive(p)) => concat(namespace, &increment_last_byte(p)),
    };
    let max = match max.map(RawBound::from) {
        None => increment_last_byte(namespace.to_vec()),
        Some(RawBound::Inclusive(p)) => concat(namespace, &increment_last_byte(p)),
        Some(RawBound::Exclusive(p)) => concat(namespace, &p),
    };

    (min, max)
}

#[cfg(test)]
mod test {
    use {super::*, crate::Borsh, grug_types::MockStorage};

    #[test]
    fn ensure_proper_range_bounds() {
        let mut store = MockStorage::new();
        // manually create this - not testing nested prefixes here
        let prefix: Prefix<Vec<u8>, u64, Borsh> = Prefix {
            namespace: b"foo".to_vec(),
            suffix: PhantomData,
            data: PhantomData,
            codec: PhantomData,
        };

        // set some data, we care about "foo" prefix
        store.write(b"foobar", b"1");
        store.write(b"foora", b"2");
        store.write(b"foozi", b"3");
        // these shouldn't match
        store.write(b"foply", b"100");
        store.write(b"font", b"200");

        let expected = vec![
            (b"bar".to_vec(), b"1".to_vec()),
            (b"ra".to_vec(), b"2".to_vec()),
            (b"zi".to_vec(), b"3".to_vec()),
        ];
        let expected_reversed: Vec<(Vec<u8>, Vec<u8>)> = expected.iter().rev().cloned().collect();

        // let's do the basic sanity check
        let res: Vec<_> = prefix
            .range_raw(&store, None, None, Order::Ascending)
            .collect();
        assert_eq!(&expected, &res);
        let res: Vec<_> = prefix
            .range_raw(&store, None, None, Order::Descending)
            .collect();
        assert_eq!(&expected_reversed, &res);

        // now let's check some ascending ranges
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                Some(Bound::inclusive(b"ra".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..], res.as_slice());
        // skip excluded
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                Some(Bound::exclusive(b"ra".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[2..], res.as_slice());
        // if we exclude something a little lower, we get matched
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                Some(Bound::exclusive(b"r".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..], res.as_slice());

        // now let's check some descending ranges
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                None,
                Some(Bound::inclusive(b"ra".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[1..], res.as_slice());
        // skip excluded
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                None,
                Some(Bound::exclusive(b"ra".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[2..], res.as_slice());
        // if we exclude something a little higher, we get matched
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                None,
                Some(Bound::exclusive(b"rb".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[1..], res.as_slice());

        // now test when both sides are set
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                Some(Bound::inclusive(b"ra".to_vec())),
                Some(Bound::exclusive(b"zi".to_vec())),
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..2], res.as_slice());
        // and descending
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                Some(Bound::inclusive(b"ra".to_vec())),
                Some(Bound::exclusive(b"zi".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected[1..2], res.as_slice());
        // Include both sides
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                Some(Bound::inclusive(b"ra".to_vec())),
                Some(Bound::inclusive(b"zi".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[..2], res.as_slice());
        // Exclude both sides
        let res: Vec<_> = prefix
            .range_raw(
                &store,
                Some(Bound::exclusive(b"ra".to_vec())),
                Some(Bound::exclusive(b"zi".to_vec())),
                Order::Ascending,
            )
            .collect();
        assert_eq!(res.as_slice(), &[]);
    }

    #[test]
    fn prefix_clear_limited() {
        let mut store = MockStorage::new();
        // manually create this - not testing nested prefixes here
        let prefix: Prefix<i32, u64, Borsh> = Prefix {
            namespace: b"foo".to_vec(),
            suffix: PhantomData,
            data: PhantomData,
            codec: PhantomData,
        };

        // set some data, we care about "foo" prefix
        for i in 0..100i32 {
            let mut buf = "foo".joined_key();
            buf.extend_from_slice(&i.joined_key());
            store.write(&buf, b"1");
        }

        assert_eq!(
            prefix.range(&store, None, None, Order::Ascending).count(),
            100
        );

        // clear with min bound
        prefix.clear(&mut store, None, Some(Bound::inclusive(20i32)));
        assert_eq!(
            prefix.range(&store, None, None, Order::Ascending).count(),
            100 - 21
        );

        // clear with max bound
        prefix.clear(&mut store, Some(Bound::inclusive(50)), None);
        assert_eq!(
            prefix.range(&store, None, None, Order::Ascending).count(),
            100 - 21 - 50
        );

        // clearing more than available should work
        prefix.clear(&mut store, None, Some(Bound::inclusive(200)));
        assert_eq!(
            prefix.range(&store, None, None, Order::Ascending).count(),
            0
        );
    }

    #[test]
    fn prefix_clear_unlimited() {
        let mut store = MockStorage::new();
        let prefix: Prefix<Vec<u8>, u64, Borsh> = Prefix::new(b"foo", &[]);

        // set some data, we care about "foo" prefix
        for i in 0..1000u32 {
            store.write(&concat(&[0, 3], format!("foo{}", i).as_bytes()), b"1");
        }

        assert_eq!(
            prefix.range(&store, None, None, Order::Ascending).count(),
            1000
        );

        // clearing all should work
        prefix.clear(&mut store, None, None);
        assert_eq!(
            prefix.range(&store, None, None, Order::Ascending).count(),
            0
        );

        // set less data
        for i in 0..5u32 {
            store.write(&concat(&[0, 3], format!("foo{}", i).as_bytes()), b"1");
        }

        assert_eq!(
            prefix.range(&store, None, None, Order::Ascending).count(),
            5
        );

        // clearing all should work
        prefix.clear(&mut store, None, None);
        assert_eq!(
            prefix.range(&store, None, None, Order::Ascending).count(),
            0
        );
    }

    #[test]
    fn is_empty_works() {
        let prefix: Prefix<Vec<u8>, u64, Borsh> = Prefix::new(b"foo", &[]);

        let mut storage = MockStorage::new();

        assert!(prefix.is_empty(&storage));

        storage.write(&concat(&[0, 3], b"fookey1"), b"1");
        storage.write(&concat(&[0, 3], b"fookey2"), b"2");

        assert!(!prefix.is_empty(&storage));
    }

    #[test]
    fn keys_raw_works() {
        let prefix: Prefix<Vec<u8>, u64, Borsh> = Prefix::new(b"foo", &[]);

        let mut storage = MockStorage::new();
        storage.write(&concat(&[0, 3], b"fookey1"), b"1");
        storage.write(&concat(&[0, 3], b"fookey2"), b"2");

        let keys: Vec<_> = prefix
            .keys_raw(&storage, None, None, Order::Ascending)
            .collect();
        assert_eq!(keys, vec![b"key1", b"key2"]);

        let keys: Vec<_> = prefix
            .keys_raw(
                &storage,
                Some(Bound::exclusive("key1")),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(keys, vec![b"key2"]);
    }
}

#[cfg(test)]
mod cosmwasm_tests {
    use {
        crate::{
            Borsh, Bound, Index, IndexList, IndexedMap, MultiIndex, PrefixBound, PrimaryKey,
            UniqueIndex,
        },
        borsh::{BorshDeserialize, BorshSerialize},
        grug_types::{BorshDeExt, BorshSerExt, MockStorage, Order, StdResult},
    };

    #[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
    struct Data {
        pub name: String,
        pub last_name: String,
        pub age: u32,
    }

    struct DataIndexes<'a> {
        // Last type parameters are for signaling pk deserialization
        pub name: MultiIndex<'a, &'a str, String, Data>,
        pub age: UniqueIndex<'a, &'a str, u32, Data>,
        pub name_lastname: UniqueIndex<'a, &'a str, (Vec<u8>, Vec<u8>), Data>,
    }

    // Future Note: this can likely be macro-derived
    impl<'a> IndexList<&'a str, Data> for DataIndexes<'a> {
        fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<&'a str, Data>> + '_> {
            let v: Vec<&dyn Index<&str, Data>> = vec![&self.name, &self.age, &self.name_lastname];
            Box::new(v.into_iter())
        }
    }

    // For composite multi index tests
    struct DataCompositeMultiIndex<'a, PK>
    where
        PK: PrimaryKey,
    {
        // Last type parameter is for signaling pk deserialization
        pub name_age: MultiIndex<'a, PK, (Vec<u8>, u32), Data>,
    }

    // Future Note: this can likely be macro-derived
    impl<'a, PK> IndexList<PK, Data> for DataCompositeMultiIndex<'a, PK>
    where
        PK: PrimaryKey,
    {
        fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PK, Data>> + '_> {
            let v: Vec<&dyn Index<PK, Data>> = vec![&self.name_age];
            Box::new(v.into_iter())
        }
    }

    const DATA: IndexedMap<&str, Data, DataIndexes> = IndexedMap::new("data", DataIndexes {
        name: MultiIndex::new(|_pk, d| d.name.to_string(), "data", "data__name"),
        age: UniqueIndex::new(|_, d| d.age, "data", "data__age"),
        name_lastname: UniqueIndex::new(
            |_, d| index_string_tuple(&d.name, &d.last_name),
            "data",
            "data__name_lastname",
        ),
    });

    fn index_string(data: &str) -> Vec<u8> {
        data.as_bytes().to_vec()
    }

    fn index_tuple(name: &str, age: u32) -> (Vec<u8>, u32) {
        (index_string(name), age)
    }

    fn index_string_tuple(data1: &str, data2: &str) -> (Vec<u8>, Vec<u8>) {
        (index_string(data1), index_string(data2))
    }

    fn save_data<'a>(store: &mut MockStorage) -> (Vec<&'a str>, Vec<Data>) {
        let mut pks = vec![];
        let mut datas = vec![];
        let data = Data {
            name: "Maria".to_string(),
            last_name: "Doe".to_string(),
            age: 42,
        };
        let pk = "1";
        DATA.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        // same name (multi-index), different last name, different age => ok
        let data = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 23,
        };
        let pk = "2";
        DATA.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        // different name, different last name, different age => ok
        let data = Data {
            name: "John".to_string(),
            last_name: "Wayne".to_string(),
            age: 32,
        };
        let pk = "3";
        DATA.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        let data = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Rodriguez".to_string(),
            age: 12,
        };
        let pk = "4";
        DATA.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        let data = Data {
            name: "Marta".to_string(),
            last_name: "After".to_string(),
            age: 90,
        };
        let pk = "5";
        DATA.save(store, pk, &data).unwrap();
        pks.push(pk);
        datas.push(data);

        (pks, datas)
    }

    #[test]
    fn store_and_load_by_index() {
        let mut store = MockStorage::new();

        // save data
        let (pks, datas) = save_data(&mut store);
        let pk = pks[0];
        let data = &datas[0];

        // load it properly
        let loaded = DATA.load(&store, pk).unwrap();
        assert_eq!(*data, loaded);

        let count = DATA
            .idx
            .name
            .prefix("Maria".to_string())
            .range_raw(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(2, count);

        // load it by secondary index
        let marias: Vec<_> = DATA
            .idx
            .name
            .prefix("Maria".to_string())
            .range_raw(&store, None, None, Order::Ascending)
            .collect();
        assert_eq!(2, marias.len());
        let (k, v) = &marias[0];
        assert_eq!(pk, String::from_slice(k).unwrap());
        assert_eq!(data, &v.deserialize_borsh::<Data>().unwrap());

        // other index doesn't match (1 byte after)
        let count = DATA
            .idx
            .name
            .prefix("Marib".to_string())
            .range_raw(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // other index doesn't match (1 byte before)
        let count = DATA
            .idx
            .name
            .prefix("Mari`".to_string())
            .range_raw(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // other index doesn't match (longer)
        let count = DATA
            .idx
            .name
            .prefix("Maria5".to_string())
            .range_raw(&store, None, None, Order::Ascending)
            .count();
        assert_eq!(0, count);

        // In a MultiIndex, the index key is composed by the index and the primary key.
        // Primary key may be empty (so that to iterate over all elements that match just the index)
        let key = ("Maria".to_string(), "");
        // Iterate using an inclusive bound over the key
        let marias = DATA
            .idx
            .name
            .range_raw(&store, Some(Bound::inclusive(key)), None, Order::Ascending)
            .collect::<Vec<_>>();
        // gets from the first "Maria" until the end
        assert_eq!(4, marias.len());

        // Build key including a non-empty pk
        let key = ("Maria".to_string(), "1");
        // Iterate using a (exclusive) bound over the key.
        // (Useful for pagination / continuation contexts).
        let count = DATA
            .idx
            .name
            .range_raw(&store, Some(Bound::exclusive(key)), None, Order::Ascending)
            .count();
        // gets from the 2nd "Maria" until the end
        assert_eq!(3, count);

        // index_key() over UniqueIndex works.
        let age_key = 23u32;
        // Iterate using a (inclusive) bound over the key.
        let count = DATA
            .idx
            .age
            .range_raw(
                &store,
                Some(Bound::inclusive(age_key)),
                None,
                Order::Ascending,
            )
            .count();
        // gets all the greater than or equal to 23 years old people
        assert_eq!(4, count);

        // match on proper age
        let proper = 42u32;
        let (k, v) = DATA.idx.age.load(&store, proper).unwrap();

        assert_eq!(pk, k);
        assert_eq!(data, &v);

        // no match on wrong age
        let too_old = 43u32;
        DATA.idx.age.load(&store, too_old).unwrap_err();
    }

    #[test]
    fn existence() {
        let mut store = MockStorage::new();
        let (pks, _) = save_data(&mut store);

        assert!(DATA.has(&store, pks[0]));
        assert!(!DATA.has(&store, "6"));
    }

    #[test]
    fn range_raw_simple_key_by_multi_index() {
        let mut store = MockStorage::new();

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk = "5627";
        DATA.save(&mut store, pk, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk = "5628";
        DATA.save(&mut store, pk, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 24,
        };
        let pk = "5629";
        DATA.save(&mut store, pk, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 12,
        };
        let pk = "5630";
        DATA.save(&mut store, pk, &data4).unwrap();

        let marias: Vec<_> = DATA
            .idx
            .name
            .prefix("Maria".to_string())
            .range_raw(&store, None, None, Order::Descending)
            .collect();
        let count = marias.len();
        assert_eq!(2, count);

        // Pks, sorted by (descending) pk
        assert_eq!(marias[0].0, b"5629");
        assert_eq!(marias[1].0, b"5627");
        // Data is correct
        assert_eq!(marias[0].1, data3.to_borsh_vec().unwrap());
        assert_eq!(marias[1].1, data1.to_borsh_vec().unwrap());
    }

    #[test]
    fn range_simple_key_by_multi_index() {
        let mut store = MockStorage::new();

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk = "5627";
        DATA.save(&mut store, pk, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk = "5628";
        DATA.save(&mut store, pk, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Williams".to_string(),
            age: 24,
        };
        let pk = "5629";
        DATA.save(&mut store, pk, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 12,
        };
        let pk = "5630";
        DATA.save(&mut store, pk, &data4).unwrap();

        let marias: Vec<_> = DATA
            .idx
            .name
            .prefix("Maria".to_string())
            .range(&store, None, None, Order::Descending)
            .collect::<StdResult<_>>()
            .unwrap();
        let count = marias.len();
        assert_eq!(2, count);

        // Pks, sorted by (descending) pk
        assert_eq!(marias[0].0, "5629");
        assert_eq!(marias[1].0, "5627");
        // Data is correct
        assert_eq!(marias[0].1, data3);
        assert_eq!(marias[1].1, data1);
    }

    #[test]
    fn range_raw_composite_key_by_multi_index() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(
                |_pk, d| index_tuple(&d.name, d.age),
                "data",
                "data__name_age",
            ),
        };
        let map: IndexedMap<&[u8], Data, DataCompositeMultiIndex<&[u8]>> =
            IndexedMap::new("data", indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1: &[u8] = b"5627";
        map.save(&mut store, pk1, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2: &[u8] = b"5628";
        map.save(&mut store, pk2, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3: &[u8] = b"5629";
        map.save(&mut store, pk3, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4: &[u8] = b"5630";
        map.save(&mut store, pk4, &data4).unwrap();

        let marias: Vec<_> = map
            .idx
            .name_age
            .sub_prefix(b"Maria".to_vec())
            .range_raw(&store, None, None, Order::Descending)
            .collect();
        let count = marias.len();
        assert_eq!(2, count);

        // Pks, sorted by (descending) age
        assert_eq!(pk1, marias[0].0);
        assert_eq!(pk3, marias[1].0);

        // Data
        assert_eq!(data1.to_borsh_vec().unwrap(), marias[0].1);
        assert_eq!(data3.to_borsh_vec().unwrap(), marias[1].1);
    }

    #[test]
    fn range_composite_key_by_multi_index() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(
                |_pk, d| index_tuple(&d.name, d.age),
                "data",
                "data__name_age",
            ),
        };
        let map: IndexedMap<&[u8], Data, DataCompositeMultiIndex<&[u8]>, Borsh> =
            IndexedMap::new("data", indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1 = b"5627";
        map.save(&mut store, pk1, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2 = b"5628";
        map.save(&mut store, pk2, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3 = b"5629";
        map.save(&mut store, pk3, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4 = b"5630";
        map.save(&mut store, pk4, &data4).unwrap();

        let marias: Vec<_> = map
            .idx
            .name_age
            .sub_prefix(b"Maria".to_vec())
            .range(&store, None, None, Order::Descending)
            .collect::<StdResult<_>>()
            .unwrap();
        let count = marias.len();
        assert_eq!(2, count);

        // Pks, sorted by (descending) age
        assert_eq!(pk1.to_vec(), marias[0].0);
        assert_eq!(pk3.to_vec(), marias[1].0);

        // Data
        assert_eq!(data1, marias[0].1);
        assert_eq!(data3, marias[1].1);
    }

    #[test]
    fn unique_index_enforced() {
        let mut store = MockStorage::new();

        // save data
        let (pks, datas) = save_data(&mut store);

        // different name, different last name, same age => error
        let data5 = Data {
            name: "Marcel".to_string(),
            last_name: "Laurens".to_string(),
            age: 42,
        };
        let pk5 = "4";

        // enforce this returns some error
        DATA.save(&mut store, pk5, &data5).unwrap_err();

        // query by unique key
        // match on proper age
        let age42 = 42u32;
        let (k, v) = DATA.idx.age.load(&store, age42).unwrap();
        assert_eq!(k, pks[0]);
        assert_eq!(v.name, datas[0].name);
        assert_eq!(v.age, datas[0].age);

        // match on other age
        let age23 = 23u32;
        let (k, v) = DATA.idx.age.load(&store, age23).unwrap();
        assert_eq!(k, pks[1]);
        assert_eq!(v.name, datas[1].name);
        assert_eq!(v.age, datas[1].age);

        // if we delete the first one, we can add the blocked one
        DATA.remove(&mut store, pks[0]).unwrap();
        DATA.save(&mut store, pk5, &data5).unwrap();
        // now 42 is the new owner
        let (k, v) = DATA.idx.age.load(&store, age42).unwrap();
        assert_eq!(k, pk5);
        assert_eq!(v.name, data5.name);
        assert_eq!(v.age, data5.age);
    }

    #[test]
    fn unique_index_enforced_composite_key() {
        let mut store = MockStorage::new();

        // save data
        save_data(&mut store);

        // same name, same lastname => error
        let data5 = Data {
            name: "Maria".to_string(),
            last_name: "Doe".to_string(),
            age: 24,
        };
        let pk5 = "5";
        // enforce this returns some error
        DATA.save(&mut store, pk5, &data5).unwrap_err();
    }

    #[test]
    fn remove_and_update_reflected_on_indexes() {
        let mut store = MockStorage::new();

        let name_count = |store: &MockStorage, name: &str| -> usize {
            DATA.idx
                .name
                .prefix(name.to_string())
                .keys_raw(store, None, None, Order::Ascending)
                .count()
        };

        // save data
        let (pks, _) = save_data(&mut store);

        // find 2 Marias, 1 John, and no Mary
        assert_eq!(name_count(&store, "Maria"), 2);
        assert_eq!(name_count(&store, "John"), 1);
        assert_eq!(name_count(&store, "Maria Luisa"), 1);
        assert_eq!(name_count(&store, "Mary"), 0);

        // remove maria 2
        DATA.remove(&mut store, pks[1]).unwrap();

        // change john to mary
        DATA.update(&mut store, pks[2], |d| -> StdResult<_> {
            let mut x = d.unwrap();
            assert_eq!(&x.name, "John");
            x.name = "Mary".to_string();
            Ok(Some(x))
        })
        .unwrap();

        // find 1 maria, 1 maria luisa, no john, and 1 mary
        assert_eq!(name_count(&store, "Maria"), 1);
        assert_eq!(name_count(&store, "Maria Luisa"), 1);
        assert_eq!(name_count(&store, "John"), 0);
        assert_eq!(name_count(&store, "Mary"), 1);
    }

    #[test]
    fn range_raw_simple_key_by_unique_index() {
        let mut store = MockStorage::new();

        // save data
        let (pks, datas) = save_data(&mut store);

        let ages: Vec<_> = DATA
            .idx
            .age
            .range_raw(&store, None, None, Order::Ascending)
            .map(|(ik, pk, v)| {
                (
                    ik,
                    String::from_slice(&pk).unwrap(),
                    v.deserialize_borsh().unwrap(),
                )
            })
            .collect();

        let count = ages.len();
        assert_eq!(5, count);

        // The ik, sorted by age ascending
        assert_eq!(datas[3].age.to_be_bytes(), ages[0].0.as_slice()); // 12
        assert_eq!(datas[1].age.to_be_bytes(), ages[1].0.as_slice()); // 23
        assert_eq!(datas[2].age.to_be_bytes(), ages[2].0.as_slice()); // 32
        assert_eq!(datas[0].age.to_be_bytes(), ages[3].0.as_slice()); // 42
        assert_eq!(datas[4].age.to_be_bytes(), ages[4].0.as_slice()); // 90

        // The pks, sorted by age ascending
        assert_eq!(pks[3], ages[0].1); // 12
        assert_eq!(pks[1], ages[1].1); // 23
        assert_eq!(pks[2], ages[2].1); // 32
        assert_eq!(pks[0], ages[3].1); // 42
        assert_eq!(pks[4], ages[4].1); // 90

        // The associated data
        assert_eq!(datas[3], ages[0].2);
        assert_eq!(datas[1], ages[1].2);
        assert_eq!(datas[2], ages[2].2);
        assert_eq!(datas[0], ages[3].2);
        assert_eq!(datas[4], ages[4].2);
    }

    #[test]
    fn range_simple_key_by_unique_index() {
        let mut store = MockStorage::new();

        // save data
        let (pks, datas) = save_data(&mut store);

        let res: StdResult<Vec<_>> = DATA
            .idx
            .age
            .range(&store, None, None, Order::Ascending)
            .collect();
        let ages = res.unwrap();

        let count = ages.len();
        assert_eq!(5, count);

        // The pks, sorted by age ascending
        assert_eq!(pks[3], ages[0].1);
        assert_eq!(pks[1], ages[1].1);
        assert_eq!(pks[2], ages[2].1);
        assert_eq!(pks[0], ages[3].1);
        assert_eq!(pks[4], ages[4].1);

        // The associated data
        assert_eq!(datas[3], ages[0].2);
        assert_eq!(datas[1], ages[1].2);
        assert_eq!(datas[2], ages[2].2);
        assert_eq!(datas[0], ages[3].2);
        assert_eq!(datas[4], ages[4].2);
    }

    // TODO: We dont' have prefix for unique index anymore

    // #[test]
    // fn range_raw_composite_key_by_unique_index() {
    //     let mut store = MockStorage::new();

    //     // save data
    //     let (pks, datas) = save_data(&mut store);

    //     let marias = DATA
    //         .idx
    //         .name_lastname
    //         .prefix(b"Maria".to_vec())
    //         .range_raw(&store, None, None, Order::Ascending)
    //         .map(|(k, v)| {
    //             (
    //                 k,
    //                 from_borsh_slice::<_, UniqueValue<&str, Data>>(&v).unwrap(),
    //             )
    //         })
    //         .collect::<Vec<_>>();

    //     // Only two people are called "Maria"
    //     let count = marias.len();
    //     assert_eq!(2, count);

    //     // The ik::suffix
    //     assert_eq!(datas[0].last_name.as_bytes(), marias[0].0);
    //     assert_eq!(datas[1].last_name.as_bytes(), marias[1].0);

    //     // The pks
    //     assert_eq!(pks[0], marias[0].1.key().unwrap());
    //     assert_eq!(pks[1], marias[1].1.key().unwrap());

    //     // The associated data
    //     assert_eq!(datas[0], marias[0].1.value);
    //     assert_eq!(datas[1], marias[1].1.value);
    // }

    // TODO: We dont' have prefix for unique index anymore

    // #[test]
    // fn range_composite_key_by_unique_index() {
    //     let mut store = MockStorage::new();

    //     // save data
    //     let (pks, datas) = save_data(&mut store);

    //     let res: StdResult<Vec<_>> = DATA
    //         .idx
    //         .name_lastname
    //         .prefix(b"Maria".to_vec())
    //         .range(&store, None, None, Order::Ascending)
    //         .collect();
    //     let marias = res.unwrap();

    //     // Only two people are called "Maria"
    //     let count = marias.len();
    //     assert_eq!(2, count);

    //     // The ik::suffix
    //     assert_eq!(datas[0].last_name.as_bytes(), marias[0].0);
    //     assert_eq!(datas[1].last_name.as_bytes(), marias[1].0);

    //     // The pks
    //     assert_eq!(pks[0], marias[0].1.key().unwrap());
    //     assert_eq!(pks[1], marias[1].1.key().unwrap());

    //     // The associated data
    //     assert_eq!(datas[0], marias[0].1.value);
    //     assert_eq!(datas[1], marias[1].1.value);
    // }

    #[test]
    fn range_simple_string_key() {
        let mut store = MockStorage::new();

        // save data
        let (pks, datas) = save_data(&mut store);

        // let's try to iterate!
        let all: StdResult<Vec<_>> = DATA.range(&store, None, None, Order::Ascending).collect();
        let all = all.unwrap();
        assert_eq!(
            all,
            pks.clone()
                .into_iter()
                .map(str::to_string)
                .zip(datas.clone().into_iter())
                .collect::<Vec<_>>()
        );

        // let's try to iterate over a range
        let all: StdResult<Vec<_>> = DATA
            .range(&store, Some(Bound::inclusive("3")), None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(
            all,
            pks.into_iter()
                .map(str::to_string)
                .zip(datas.into_iter())
                .rev()
                .take(3)
                .rev()
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn prefix_simple_string_key() {
        let mut store = MockStorage::new();

        // save data
        let (pks, datas) = save_data(&mut store);

        // Let's prefix and iterate.
        // This is similar to calling range() directly, but added here for completeness / prefix
        // type checks

        // Grug note:
        // we changed this. This test doesn't make sense now.
        // it's like to call only range. With prefix, the IK::Prefix in this case is ().
        let all: StdResult<Vec<_>> = DATA
            // .prefix(())
            .range(&store, None, None, Order::Ascending)
            .collect();
        let all = all.unwrap();
        assert_eq!(
            all,
            pks.clone()
                .into_iter()
                .map(str::to_string)
                .zip(datas.into_iter())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn prefix_composite_key() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex::<(&str, &str)> {
            name_age: MultiIndex::new(
                |_pk, d| index_tuple(&d.name, d.age),
                "data",
                "data__name_age",
            ),
        };
        let map: IndexedMap<(&str, &str), Data, DataCompositeMultiIndex<(&str, &str)>, Borsh> =
            IndexedMap::new("data", indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1 = ("1", "5627");
        map.save(&mut store, pk1, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2 = ("2", "5628");
        map.save(&mut store, pk2, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3 = ("2", "5629");
        map.save(&mut store, pk3, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4 = ("3", "5630");
        map.save(&mut store, pk4, &data4).unwrap();

        // let's prefix and iterate
        let result: StdResult<Vec<_>> = map
            .prefix("2")
            .range(&store, None, None, Order::Ascending)
            .collect();
        let result = result.unwrap();
        assert_eq!(result, [
            ("5628".to_string(), data2),
            ("5629".to_string(), data3),
        ]);
    }

    #[test]
    fn prefix_triple_key() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(
                |_pk, d| index_tuple(&d.name, d.age),
                "data",
                "data__name_age",
            ),
        };
        let map: IndexedMap<(&str, &str, &str), Data, DataCompositeMultiIndex<(&str, &str, &str)>> =
            IndexedMap::new("data", indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1 = ("1", "1", "5627");
        map.save(&mut store, pk1, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2 = ("1", "2", "5628");
        map.save(&mut store, pk2, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3 = ("2", "1", "5629");
        map.save(&mut store, pk3, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4 = ("2", "2", "5630");
        map.save(&mut store, pk4, &data4).unwrap();

        // let's prefix and iterate
        let result: StdResult<Vec<_>> = map
            .prefix("1")
            .append("2")
            .range(&store, None, None, Order::Ascending)
            .collect();
        let result = result.unwrap();
        assert_eq!(result, [("5628".to_string(), data2),]);
    }

    #[test]
    fn sub_prefix_triple_key() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(
                |_pk, d| index_tuple(&d.name, d.age),
                "data",
                "data__name_age",
            ),
        };
        let map: IndexedMap<(&str, &str, &str), Data, DataCompositeMultiIndex<(&str, &str, &str)>> =
            IndexedMap::new("data", indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1 = ("1", "1", "5627");
        map.save(&mut store, pk1, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2 = ("1", "2", "5628");
        map.save(&mut store, pk2, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3 = ("2", "1", "5629");
        map.save(&mut store, pk3, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4 = ("2", "2", "5630");
        map.save(&mut store, pk4, &data4).unwrap();

        // let's sub-prefix and iterate
        let result: StdResult<Vec<_>> = map
            .prefix("1")
            .range(&store, None, None, Order::Ascending)
            .collect();
        let result = result.unwrap();
        assert_eq!(result, [
            (("1".to_string(), "5627".to_string()), data1),
            (("2".to_string(), "5628".to_string()), data2),
        ]);
    }

    #[test]
    fn prefix_range_simple_key() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(
                |_pk, d| index_tuple(&d.name, d.age),
                "data",
                "data__name_age",
            ),
        };
        let map: IndexedMap<(&str, &str), Data, DataCompositeMultiIndex<(&str, &str)>> =
            IndexedMap::new("data", indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1 = ("1", "5627");
        map.save(&mut store, pk1, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2 = ("2", "5628");
        map.save(&mut store, pk2, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3 = ("2", "5629");
        map.save(&mut store, pk3, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4 = ("3", "5630");
        map.save(&mut store, pk4, &data4).unwrap();

        // let's prefix-range and iterate
        let result: StdResult<Vec<_>> = map
            .prefix_range(
                &store,
                Some(PrefixBound::inclusive("2")),
                None,
                Order::Ascending,
            )
            .collect();
        let result = result.unwrap();
        assert_eq!(result, [
            (("2".to_string(), "5628".to_string()), data2.clone()),
            (("2".to_string(), "5629".to_string()), data3.clone()),
            (("3".to_string(), "5630".to_string()), data4)
        ]);

        // let's try to iterate over a more restrictive prefix-range!
        let result: StdResult<Vec<_>> = map
            .prefix_range(
                &store,
                Some(PrefixBound::inclusive("2")),
                Some(PrefixBound::exclusive("3")),
                Order::Ascending,
            )
            .collect();
        let result = result.unwrap();
        assert_eq!(result, [
            (("2".to_string(), "5628".to_string()), data2),
            (("2".to_string(), "5629".to_string()), data3),
        ]);
    }

    #[test]
    fn prefix_range_triple_key() {
        let mut store = MockStorage::new();

        let indexes = DataCompositeMultiIndex {
            name_age: MultiIndex::new(
                |_pk, d| index_tuple(&d.name, d.age),
                "data",
                "data__name_age",
            ),
        };
        let map: IndexedMap<(&str, &str, &str), Data, DataCompositeMultiIndex<(&str, &str, &str)>> =
            IndexedMap::new("data", indexes);

        // save data
        let data1 = Data {
            name: "Maria".to_string(),
            last_name: "".to_string(),
            age: 42,
        };
        let pk1 = ("1", "1", "5627");
        map.save(&mut store, pk1, &data1).unwrap();

        let data2 = Data {
            name: "Juan".to_string(),
            last_name: "Perez".to_string(),
            age: 13,
        };
        let pk2 = ("1", "2", "5628");
        map.save(&mut store, pk2, &data2).unwrap();

        let data3 = Data {
            name: "Maria".to_string(),
            last_name: "Young".to_string(),
            age: 24,
        };
        let pk3 = ("2", "1", "5629");
        map.save(&mut store, pk3, &data3).unwrap();

        let data4 = Data {
            name: "Maria Luisa".to_string(),
            last_name: "Bemberg".to_string(),
            age: 43,
        };
        let pk4 = ("2", "2", "5630");
        map.save(&mut store, pk4, &data4).unwrap();

        // Grug implementation:
        // on grug the prefix for (A, B, C) is A.
        // Cosmwasm has (A, B) as prefix.

        // let's prefix-range and iterate
        let result: StdResult<Vec<_>> = map
            .prefix_range(
                &store,
                Some(PrefixBound::inclusive("1")),
                None,
                Order::Ascending,
            )
            .collect();
        let result = result.unwrap();
        assert_eq!(result, [
            (
                ("1".to_string(), "1".to_string(), "5627".to_string()),
                data1.clone()
            ),
            (
                ("1".to_string(), "2".to_string(), "5628".to_string()),
                data2.clone()
            ),
            (
                ("2".to_string(), "1".to_string(), "5629".to_string()),
                data3.clone()
            ),
            (
                ("2".to_string(), "2".to_string(), "5630".to_string()),
                data4
            )
        ]);

        // let's prefix-range over inclusive bounds on both sides
        let result: StdResult<Vec<_>> = map
            .prefix_range(
                &store,
                Some(PrefixBound::inclusive("1")),
                Some(PrefixBound::exclusive("2")),
                Order::Ascending,
            )
            .collect();
        let result = result.unwrap();
        assert_eq!(result, [
            (
                ("1".to_string(), "1".to_string(), "5627".to_string()),
                data1.clone()
            ),
            (
                ("1".to_string(), "2".to_string(), "5628".to_string()),
                data2.clone()
            ),
        ]);
    }

    mod bounds_unique_index {
        use super::*;

        struct Indexes<'a, PK: PrimaryKey> {
            secondary: UniqueIndex<'a, PK, u64, u64>,
        }

        impl<'a, PK> IndexList<PK, u64> for Indexes<'a, PK>
        where
            PK: PrimaryKey,
        {
            fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<PK, u64>> + '_> {
                let v: Vec<&dyn Index<PK, u64>> = vec![&self.secondary];
                Box::new(v.into_iter())
            }
        }

        #[test]
        fn composite_key_query() {
            let indexes = Indexes {
                secondary: UniqueIndex::new(
                    |_, secondary| *secondary,
                    "test_map",
                    "test_map__secondary",
                ),
            };
            let map = IndexedMap::<&str, u64, Indexes<&str>>::new("test_map", indexes);
            let mut store = MockStorage::new();

            map.save(&mut store, "one", &1).unwrap();
            map.save(&mut store, "two", &2).unwrap();
            map.save(&mut store, "three", &3).unwrap();

            // Inclusive bound
            let items: Vec<_> = map
                .idx
                .secondary
                .values(&store, None, Some(Bound::inclusive(1u64)), Order::Ascending)
                .map(|val| val.unwrap().1)
                .collect();

            // Strip the index from values (for simpler comparison)
            // let items: Vec<_> = items.into_iter().map(|(_, v)| v).collect();

            assert_eq!(items, vec![1]);

            // Exclusive bound
            let items: Vec<_> = map
                .idx
                .secondary
                .values(&store, Some(Bound::exclusive(2u64)), None, Order::Ascending)
                .map(|val| val.unwrap().1)
                .collect();

            assert_eq!(items, vec![3]);
        }
    }

    mod bounds_multi_index {
        use super::*;

        struct Indexes<'a> {
            // The last type param must match the `IndexedMap` primary key type, below
            secondary: MultiIndex<'a, &'a str, u64, u64>,
        }

        impl<'a> IndexList<&'a str, u64> for Indexes<'a> {
            fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<&'a str, u64>> + '_> {
                let v: Vec<&dyn Index<&str, u64>> = vec![&self.secondary];
                Box::new(v.into_iter())
            }
        }

        #[test]
        fn composite_key_query() {
            let indexes = Indexes {
                secondary: MultiIndex::new(
                    |_pk, secondary| *secondary,
                    "test_map",
                    "test_map__secondary",
                ),
            };
            let map = IndexedMap::<&str, u64, Indexes>::new("test_map", indexes);
            let mut store = MockStorage::new();

            map.save(&mut store, "one", &1).unwrap();
            map.save(&mut store, "two", &2).unwrap();
            map.save(&mut store, "two2", &2).unwrap();
            map.save(&mut store, "three", &3).unwrap();

            // TODO: Grug note: we don't have prefix_range_raw implemented
            // leaving this here for future implementation

            // Inclusive prefix-bound
            // let items: Vec<_> = map
            //     .idx
            //     .secondary
            //     .prefix_range_raw()
            //     .range(&store, None, Some(Bound::inclusive(1u64)), Order::Ascending)
            //     .collect::<Result<_, _>>()
            //     .unwrap();

            // Strip the index from values (for simpler comparison)
            // let items: Vec<_> = items.into_iter().map(|(_, v)| v).collect();

            // assert_eq!(items, vec![1]);

            // Exclusive bound (used for pagination)
            // Range over the index specifying a primary key (multi-index key includes the pk)
            let items: Vec<_> = map
                .idx
                .secondary
                .range(
                    &store,
                    Some(Bound::exclusive((2u64, "two"))),
                    None,
                    Order::Ascending,
                )
                .collect::<Result<_, _>>()
                .unwrap();

            assert_eq!(items, vec![
                (2, "two2".to_string(), 2),
                (3, "three".to_string(), 3)
            ]);
        }
    }

    mod pk_multi_index {
        use {
            super::*,
            grug_types::{Addr, Uint128},
        };

        struct Indexes<'a> {
            // The last type param must match the `IndexedMap` primary key type below
            spender: MultiIndex<'a, (&'a Addr, &'a Addr), Addr, Uint128>,
        }

        impl<'a> IndexList<(&'a Addr, &'a Addr), Uint128> for Indexes<'a> {
            fn get_indexes(
                &'_ self,
            ) -> Box<dyn Iterator<Item = &'_ dyn Index<(&'a Addr, &'a Addr), Uint128>> + '_>
            {
                let v: Vec<&dyn Index<(&Addr, &Addr), Uint128>> = vec![&self.spender];
                Box::new(v.into_iter())
            }
        }

        #[test]
        fn pk_based_index() {
            let indexes = Indexes {
                spender: MultiIndex::new(|pk, _allow| *pk.1, "allowances", "allowances__spender"),
            };
            let map: IndexedMap<(&Addr, &Addr), grug_types::Uint<u128>, Indexes> =
                IndexedMap::new("allowances", indexes);
            let mut store = MockStorage::new();

            let owner_1 = Addr::mock(1);
            let owner_2 = Addr::mock(2);
            let spender_1 = Addr::mock(3);
            let spender_2 = Addr::mock(4);

            map.save(&mut store, (&owner_1, &spender_1), &Uint128::new(11))
                .unwrap();
            map.save(&mut store, (&owner_1, &spender_2), &Uint128::new(12))
                .unwrap();
            map.save(&mut store, (&owner_2, &spender_1), &Uint128::new(21))
                .unwrap();

            // Iterate over the main values
            let items: Vec<_> = map
                .range_raw(&store, None, None, Order::Ascending)
                .collect();

            // Strip the index from values (for simpler comparison)
            let items: Vec<u128> = items
                .into_iter()
                .map(|(_, v)| v.deserialize_borsh::<Uint128>().unwrap().into())
                .collect();

            assert_eq!(items, vec![11, 12, 21]);

            // Iterate over the indexed values
            let items = map
                .idx
                .spender
                .range(&store, None, None, Order::Ascending)
                .map(|val| val.unwrap().2)
                .collect::<Vec<_>>();

            assert_eq!(items, vec![11_u128.into(), 21_u128.into(), 12_u128.into()]);

            // Prefix over the main values
            let items: Vec<_> = map
                .prefix(&owner_1)
                .range(&store, None, None, Order::Ascending)
                .collect::<StdResult<_>>()
                .unwrap();

            // Strip the index from values (for simpler comparison)
            let items: Vec<u128> = items.into_iter().map(|(_, v)| v.into()).collect();

            assert_eq!(items, vec![11, 12]);

            // Prefix over the indexed values
            let items: Vec<_> = map
                .idx
                .spender
                .prefix(spender_1)
                .range(&store, None, None, Order::Ascending)
                .collect::<Result<_, _>>()
                .unwrap();

            // Strip the index from values (for simpler comparison)
            let items: Vec<u128> = items.into_iter().map(|(_, v)| v.into()).collect();

            assert_eq!(items, vec![11, 21]);

            // Prefix over the indexed values, and deserialize primary key as well
            let items: Vec<_> = map
                .idx
                .spender
                .prefix(spender_2)
                .range(&store, None, None, Order::Ascending)
                .collect::<Result<_, _>>()
                .unwrap();

            assert_eq!(items, vec![((owner_1, spender_2), Uint128::new(12))]);
        }
    }

    #[test]
    fn clear_works() {
        let mut storage = MockStorage::new();
        let (pks, _) = save_data(&mut storage);

        DATA.clear(&mut storage, None, None);

        for key in pks {
            assert!(!DATA.has(&storage, key));
        }
    }

    #[test]
    fn is_empty_works() {
        let mut storage = MockStorage::new();

        assert!(DATA.is_empty(&storage));

        save_data(&mut storage);

        assert!(!DATA.is_empty(&storage));
    }
}
