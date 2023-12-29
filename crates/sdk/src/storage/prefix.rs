use {
    super::{concat, extend_one_byte, increment_last_byte, nested_namespaces_with_key, trim},
    crate::{from_json, MapKey, Order, RawKey, Storage},
    serde::de::DeserializeOwned,
    std::{marker::PhantomData, ops::Bound},
};

pub struct Prefix<K, T> {
    prefix:       Vec<u8>,
    _suffix_type: PhantomData<K>,
    _data_type:   PhantomData<T>,
}

impl<K, T> Prefix<K, T> {
    pub fn new(namespace: &[u8], prefixes: &[RawKey]) -> Self {
        Self {
            prefix:       nested_namespaces_with_key(Some(namespace), prefixes, None),
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
    pub fn range<'a>(
        &self,
        store: &'a dyn Storage,
        min:   Bound<&K>,
        max:   Bound<&K>,
        order: Order,
    ) -> Box<dyn Iterator<Item = anyhow::Result<(K, T)>> + 'a> {
        // compute start and end bounds
        // note that the store considers the start bounds as inclusive, and end
        // bound as exclusive (see the Storage trait)
        let min = match min {
            Bound::Unbounded => self.prefix.to_vec(),
            Bound::Included(k) => concat(&self.prefix, &k.serialize()),
            Bound::Excluded(k) => extend_one_byte(concat(&self.prefix, &k.serialize())),
        };
        let max = match max {
            Bound::Unbounded => increment_last_byte(self.prefix.to_vec()),
            Bound::Included(k) => extend_one_byte(concat(&self.prefix, &k.serialize())),
            Bound::Excluded(k) => concat(&self.prefix, &k.serialize()),
        };

        // need to make a clone of self.prefix and move it into the closure,
        // so that the iterator can live longer than &self.
        let prefix = self.prefix.clone();
        let iter = store.scan(Some(&min), Some(&max), order).map(move |(k, v)| {
            debug_assert_eq!(&k[0..prefix.len()], prefix, "Prefix mispatch");
            let key_bytes = trim(&prefix, &k);
            let key = K::deserialize(&key_bytes)?;
            let data = from_json(&v)?;
            Ok((key, data))
        });

        Box::new(iter)
    }

    pub fn clear(&self, _store: &mut dyn Storage, _limit: Option<usize>) {
        todo!()
    }
}
