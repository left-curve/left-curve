use {
    crate::{
        concat, extend_one_byte, from_json, increment_last_byte, nested_namespaces_with_key, trim,
        Bound, MapKey, Order, RawBound, RawKey, StdResult, Storage,
    },
    serde::de::DeserializeOwned,
    std::marker::PhantomData,
};

pub struct Prefix<K, T> {
    prefix:       Vec<u8>,
    _suffix_type: PhantomData<K>,
    _data_type:   PhantomData<T>,
}

impl<K, T> Prefix<K, T> {
    pub fn new(namespace: &[u8], prefixes: &[RawKey]) -> Self {
        Self {
            prefix: nested_namespaces_with_key(Some(namespace), prefixes, <Option<&RawKey>>::None),
            _suffix_type: PhantomData,
            _data_type:   PhantomData,
        }
    }
}

impl<K, T> Prefix<K, T>
where
    K: MapKey,
    T: DeserializeOwned,
{
    #[allow(clippy::type_complexity)]
    pub fn range<'a>(
        &self,
        store: &'a dyn Storage,
        min:   Option<Bound<K>>,
        max:   Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'a> {
        // compute start and end bounds
        // note that the store considers the start bounds as inclusive, and end
        // bound as exclusive (see the Storage trait)
        let (min, max) = range_bounds(&self.prefix, min, max);

        // need to make a clone of self.prefix and move it into the closure,
        // so that the iterator can live longer than &self.
        let prefix = self.prefix.clone();
        let iter = store.scan(Some(&min), Some(&max), order).map(move |(k, v)| {
            debug_assert_eq!(&k[0..prefix.len()], prefix, "Prefix mispatch");
            let key_bytes = trim(&prefix, &k);
            let key = K::deserialize(&key_bytes)?;
            let data = from_json(v)?;
            Ok((key, data))
        });

        Box::new(iter)
    }

    pub fn keys<'a>(
        &self,
        store: &'a dyn Storage,
        min:   Option<Bound<K>>,
        max:   Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'a> {
        let (min, max) = range_bounds(&self.prefix, min, max);
        let prefix = self.prefix.clone();
        // TODO: this is really inefficient because the host needs to load both
        // the key and value into Wasm memory
        let iter = store.scan(Some(&min), Some(&max), order).map(move |(k, _)| {
            debug_assert_eq!(&k[0..prefix.len()], prefix, "prefix mispatch");
            let key_bytes = trim(&prefix, &k);
            K::deserialize(&key_bytes)
        });
        Box::new(iter)
    }

    pub fn clear(
        &self,
        _store: &mut dyn Storage,
        _min:   Option<Bound<K>>,
        _max:   Option<Bound<K>>,
        _limit: Option<usize>,
    ) -> StdResult<()> {
        todo!()
    }
}

fn range_bounds<K: MapKey>(
    prefix: &[u8],
    min:    Option<Bound<K>>,
    max:    Option<Bound<K>>,
) -> (Vec<u8>, Vec<u8>) {
    let min = match min.map(RawBound::from) {
        None => prefix.to_vec(),
        Some(RawBound::Inclusive(k)) => concat(prefix, &k),
        Some(RawBound::Exclusive(k)) => extend_one_byte(concat(prefix, &k)),
    };
    let max = match max.map(RawBound::from) {
        None => increment_last_byte(prefix.to_vec()),
        Some(RawBound::Inclusive(k)) => extend_one_byte(concat(prefix, &k)),
        Some(RawBound::Exclusive(k)) => concat(prefix, &k),
    };

    (min, max)
}
