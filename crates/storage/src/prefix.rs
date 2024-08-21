use {
    crate::{Bound, Codec, PrefixBound, Prefixer, PrimaryKey, RawBound},
    grug_types::{
        concat, encode_length, extend_one_byte, increment_last_byte, nested_namespaces_with_key,
        trim, Order, Record, StdResult, Storage,
    },
    std::{borrow::Cow, marker::PhantomData},
};

/// Generic type `I` for [`Prefix`] for enable `range/values` methods.
pub struct RangeIterator;

pub struct Prefix<K, T, C, I = RangeIterator>
where
    C: Codec<T>,
{
    namespace: Vec<u8>,
    phantom: PhantomData<(K, T, C, I)>,
}

impl<K, T, C, I> Prefix<K, T, C, I>
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
            phantom: PhantomData,
        }
    }
}

impl<K, T, C, I> Prefix<K, T, C, I>
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
            phantom: PhantomData,
        }
    }

    // -------------------- iteration methods (full bound) ---------------------

    pub fn clear(&self, storage: &mut dyn Storage, min: Option<Bound<K>>, max: Option<Bound<K>>) {
        let (min, max) = range_bounds(&self.namespace, min, max);
        storage.remove_range(Some(&min), Some(&max))
    }

    // ------------------- iteration methods (prefix bound) --------------------

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

// ----------------------------------- keys iterator -----------------------------------

impl<K, T, C, I> Prefix<K, T, C, I>
where
    K: Key,
    C: Codec<T>,
{
    pub fn is_empty(&self, storage: &dyn Storage) -> bool {
        self.keys_raw(storage, None, None, Order::Ascending)
            .next()
            .is_none()
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
}

// ----------------------------------- range iterator -----------------------------------

impl<K, T, C> Prefix<K, T, C, RangeIterator>
where
    K: Key,
    C: Codec<T>,
{
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
}

// ------

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
