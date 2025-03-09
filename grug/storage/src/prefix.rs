use {
    crate::{Codec, PrefixBound, Prefixer, PrimaryKey, RawBound, RawKey},
    grug_types::{
        Bound, Order, Record, StdResult, Storage, concat, encode_length, extend_one_byte,
        increment_last_byte, nested_namespaces_with_key, trim,
    },
    std::{collections::BTreeMap, marker::PhantomData},
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
    pub fn new(namespace: &[u8], prefixes: &[RawKey]) -> Self {
        Self {
            namespace: nested_namespaces_with_key(
                Some(namespace),
                prefixes,
                Option::<RawKey>::None,
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
        // Note that the storage considers the start bounds as inclusive, and end
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

    // TODO: this isn't very optimized because we can `range_bounds` function
    // twice, once in `self.range`, once in `self.clear`. Optimize this to only
    // call it once.
    pub fn drain(
        &self,
        storage: &mut dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
    ) -> StdResult<BTreeMap<K::Output, T>>
    where
        K: Clone,
        K::Output: Ord,
    {
        // The iteration order here doesn't matter, because we're collecting the
        // data into a `BTreeMap`, which naturally comes sorted.
        let data = self
            .range(storage, min.clone(), max.clone(), Order::Ascending)
            .collect();

        self.clear(storage, min, max);

        data
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

    pub(crate) fn prefix_keys_raw_no_trim<'a>(
        &self,
        storage: &'a dyn Storage,
        min: Option<PrefixBound<K>>,
        max: Option<PrefixBound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'a> {
        let (min, max) = range_prefix_bounds(&self.namespace, min, max);
        let iter = storage.scan_keys(Some(&min), Some(&max), order);

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

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod test {
    use {super::*, crate::Borsh, grug_types::MockStorage};

    #[test]
    fn ensure_proper_range_bounds() {
        let mut storage = MockStorage::new();

        // manually create this - not testing nested prefixes here
        let prefix: Prefix<Vec<u8>, u64, Borsh> = Prefix {
            namespace: b"foo".to_vec(),
            suffix: PhantomData,
            data: PhantomData,
            codec: PhantomData,
        };

        // set some data, we care about "foo" prefix
        storage.write(b"foobar", b"1");
        storage.write(b"foora", b"2");
        storage.write(b"foozi", b"3");
        // these shouldn't match
        storage.write(b"foply", b"100");
        storage.write(b"font", b"200");

        let expected = vec![
            (b"bar".to_vec(), b"1".to_vec()),
            (b"ra".to_vec(), b"2".to_vec()),
            (b"zi".to_vec(), b"3".to_vec()),
        ];
        let expected_reversed: Vec<_> = expected.iter().rev().cloned().collect();

        // let's do the basic sanity check
        let res: Vec<_> = prefix
            .range_raw(&storage, None, None, Order::Ascending)
            .collect();
        assert_eq!(&expected, &res);

        let res: Vec<_> = prefix
            .range_raw(&storage, None, None, Order::Descending)
            .collect();
        assert_eq!(&expected_reversed, &res);

        // now let's check some ascending ranges
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                Some(Bound::Inclusive(b"ra".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..], res.as_slice());

        // skip excluded
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                Some(Bound::Exclusive(b"ra".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[2..], res.as_slice());

        // if we exclude something a little lower, we get matched
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                Some(Bound::Exclusive(b"r".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..], res.as_slice());

        // now let's check some descending ranges
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                None,
                Some(Bound::Inclusive(b"ra".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[1..], res.as_slice());

        // skip excluded
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                None,
                Some(Bound::Exclusive(b"ra".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[2..], res.as_slice());

        // if we exclude something a little higher, we get matched
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                None,
                Some(Bound::Exclusive(b"rb".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[1..], res.as_slice());

        // now test when both sides are set
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                Some(Bound::Inclusive(b"ra".to_vec())),
                Some(Bound::Exclusive(b"zi".to_vec())),
                Order::Ascending,
            )
            .collect();
        assert_eq!(&expected[1..2], res.as_slice());

        // and descending
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                Some(Bound::Inclusive(b"ra".to_vec())),
                Some(Bound::Exclusive(b"zi".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected[1..2], res.as_slice());

        // Include both sides
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                Some(Bound::Inclusive(b"ra".to_vec())),
                Some(Bound::Inclusive(b"zi".to_vec())),
                Order::Descending,
            )
            .collect();
        assert_eq!(&expected_reversed[..2], res.as_slice());

        // Exclude both sides
        let res: Vec<_> = prefix
            .range_raw(
                &storage,
                Some(Bound::Exclusive(b"ra".to_vec())),
                Some(Bound::Exclusive(b"zi".to_vec())),
                Order::Ascending,
            )
            .collect();
        assert_eq!(res.as_slice(), &[]);
    }

    #[test]
    fn prefix_clear_limited() {
        let mut storage = MockStorage::new();
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
            storage.write(&buf, b"1");
        }

        assert_eq!(
            prefix.range(&storage, None, None, Order::Ascending).count(),
            100
        );

        // clear with min bound
        prefix.clear(&mut storage, None, Some(Bound::Inclusive(20i32)));
        assert_eq!(
            prefix.range(&storage, None, None, Order::Ascending).count(),
            100 - 21
        );

        // clear with max bound
        prefix.clear(&mut storage, Some(Bound::Inclusive(50)), None);
        assert_eq!(
            prefix.range(&storage, None, None, Order::Ascending).count(),
            100 - 21 - 50
        );

        // clearing more than available should work
        prefix.clear(&mut storage, None, Some(Bound::Inclusive(200)));
        assert_eq!(
            prefix.range(&storage, None, None, Order::Ascending).count(),
            0
        );
    }

    #[test]
    fn prefix_clear_unlimited() {
        let mut storage = MockStorage::new();
        let prefix: Prefix<Vec<u8>, u64, Borsh> = Prefix::new(b"foo", &[]);

        // set some data, we care about "foo" prefix
        for i in 0..1000u32 {
            storage.write(&concat(&[0, 3], format!("foo{}", i).as_bytes()), b"1");
        }

        assert_eq!(
            prefix.range(&storage, None, None, Order::Ascending).count(),
            1000
        );

        // clearing all should work
        prefix.clear(&mut storage, None, None);
        assert_eq!(
            prefix.range(&storage, None, None, Order::Ascending).count(),
            0
        );

        // set less data
        for i in 0..5u32 {
            storage.write(&concat(&[0, 3], format!("foo{}", i).as_bytes()), b"1");
        }

        assert_eq!(
            prefix.range(&storage, None, None, Order::Ascending).count(),
            5
        );

        // clearing all should work
        prefix.clear(&mut storage, None, None);
        assert_eq!(
            prefix.range(&storage, None, None, Order::Ascending).count(),
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
                Some(Bound::Exclusive(b"key1".to_vec())),
                None,
                Order::Ascending,
            )
            .collect();
        assert_eq!(keys, vec![b"key2"]);
    }
}
