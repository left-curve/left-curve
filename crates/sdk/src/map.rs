use {
    crate::{from_json, to_json, MapKey, Storage},
    anyhow::Context,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{any::type_name, marker::PhantomData},
};

pub struct Map<K, T> {
    namespace:  &'static [u8],
    _key_type:  PhantomData<K>,
    _data_type: PhantomData<T>,
}

impl<K, T> Map<K, T> {
    pub const fn new(namespace: &'static str) -> Self {
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
    T: Serialize + DeserializeOwned,
{
    pub fn is_empty(&self, store: &dyn Storage) -> bool {
        todo!()
    }

    pub fn has(&self, store: &dyn Storage, k: &K) -> bool {
        store.read(&self.path(k)).is_some()
    }

    pub fn may_load(&self, store: &dyn Storage, k: &K) -> anyhow::Result<Option<T>> {
        store
            .read(&self.path(k))
            .map(from_json)
            .transpose()
            .map_err(Into::into)
    }

    pub fn load(&self, store: &dyn Storage, k: &K) -> anyhow::Result<T> {
        let path = self.path(k);
        from_json(&store
            .read(&path)
            .with_context(|| format!(
                "[Map]: data not found! key type: {}, data type: {}, path: {}",
                type_name::<K>(),
                type_name::<T>(),
                hex::encode(&path),
            ))?)
            .map_err(Into::into)
    }

    pub fn update<A>(&self, store: &mut dyn Storage, k: &K, action: A) -> anyhow::Result<Option<T>>
    where
        A: FnOnce(Option<T>) -> anyhow::Result<Option<T>>
    {
        let maybe_data = action(self.may_load(store, k)?)?;

        if let Some(data) = &maybe_data {
            self.save(store, k, data)?;
        } else {
            self.remove(store, k);
        }

        Ok(maybe_data)
    }

    pub fn save(&self, store: &mut dyn Storage, k: &K, data: &T) -> anyhow::Result<()> {
        let bytes = to_json(data)?;
        store.write(&self.path(k), &bytes);
        Ok(())
    }

    pub fn remove(&self, store: &mut dyn Storage, k: &K) {
        store.remove(&self.path(k))
    }

    pub fn clear(&self, store: &mut dyn Storage) {
        todo!()
    }

    pub fn range<'a>(
        &self,
        store:       &dyn Storage,
        start_after: Option<&K>,
        limit:       Option<u32>,
    ) -> Box<dyn Iterator<Item = anyhow::Result<(K, T)>> + 'a> {
        todo!()
    }

    pub fn prefix<'a>(
        &self,
        store:       &dyn Storage,
        prefix:      &K::Prefix,
        start_after: Option<&K::Suffix>,
        limit:       Option<u32>,
    ) -> Box<dyn Iterator<Item = anyhow::Result<(K::Suffix, T)>> + 'a> {
        todo!()
    }

    /// Combine a namespace a one or more keys into a full byte path.
    ///
    /// The namespace and all keys other than the last one is prefixed with
    /// their lengths (2 bytes big-endian). This helps us know where a key ends
    /// and where the next key starts.
    ///
    /// E.g. if keys are [key1, key2, key3], the resulting byte path is:
    /// len(namespace) | namespace | len(key1) | key1 | len(key2) | key2 | key3
    ///
    /// Panics on two situations:
    /// - keys array is empty (must have at least one key)
    /// - any key's length exceeds u16::MAX (because we need to put the length
    ///   into 2 bytes)
    fn path(&self, key: &K) -> Vec<u8> {
        let mut keys = key.serialize();
        let last_key = keys.pop().unwrap_or_else(|| {
            panic!("Input slice is empty");
        });

        // compute the total length, so that we can allocate a Vec with the
        // necessary capacity at once, without having to reallocate
        let mut size = 2 + self.namespace.len() + last_key.len();
        for key in &keys {
            size += 2 + key.len();
        }

        // allocate the Vec and fill in the bytes
        let mut combined = Vec::with_capacity(size);
        combined.extend_from_slice(&encode_length(self.namespace));
        combined.extend_from_slice(self.namespace);
        for key in keys {
            combined.extend_from_slice(&encode_length(&key));
            combined.extend_from_slice(key.as_ref());
        }
        combined.extend_from_slice(last_key.as_ref());
        combined
    }
}

/// Return the length of the byte slice as two big-endian bytes.
/// Panic if the length is bigger than u16::MAX.
fn encode_length(bytes: impl AsRef<[u8]>) -> [u8; 2] {
    let len = bytes.as_ref().len();
    if len > 0xffff {
        panic!("Can't encode length becayse byte slice is too long: {} > {}", len, u16::MAX);
    }

    (bytes.as_ref().len() as u16).to_be_bytes()
}
