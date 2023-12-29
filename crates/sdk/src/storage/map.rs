use {
    crate::{from_json, to_json, MapKey, Order, Storage},
    anyhow::Context,
    data_encoding::BASE64,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{any::type_name, marker::PhantomData, ops::Bound},
};

pub struct Map<K, T> {
    namespace:  &'static [u8],
    _key_type:  PhantomData<K>,
    _data_type: PhantomData<T>,
}

impl<K, T> Map<K, T> {
    pub const fn new(namespace: &'static str) -> Self {
        // TODO: add a maximum length for namespace
        // see comments of increment_last_byte function for rationale
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
        store.read(&path(self.namespace, k)).is_some()
    }

    pub fn may_load(&self, store: &dyn Storage, k: &K) -> anyhow::Result<Option<T>> {
        store
            .read(&path(self.namespace, k))
            .map(from_json)
            .transpose()
            .map_err(Into::into)
    }

    pub fn load(&self, store: &dyn Storage, k: &K) -> anyhow::Result<T> {
        let path = path(self.namespace, k);
        from_json(&store
            .read(&path)
            .with_context(|| format!(
                "[Map]: data not found! key type: {}, data type: {}, path: {}",
                type_name::<K>(),
                type_name::<T>(),
                BASE64.encode(&path),
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
        store.write(&path(self.namespace, k), &bytes);
        Ok(())
    }

    pub fn remove(&self, store: &mut dyn Storage, k: &K) {
        store.remove(&path(self.namespace, k))
    }

    pub fn clear(&self, store: &mut dyn Storage) {
        todo!()
    }

    pub fn range<'a>(
        &'a self,
        store: &'a dyn Storage,
        min:   Bound<&K>,
        max:   Bound<&K>,
        order: Order,
    ) -> Box<dyn Iterator<Item = anyhow::Result<(K, T)>> + 'a> {
        // compute start and end bounds
        // note that the store considers the start bounds as inclusive, and end
        // bound as exclusive (see the Storage trait)
        let min = match min {
            Bound::Unbounded => prefix_length(self.namespace),
            Bound::Included(k) => path(self.namespace, k),
            Bound::Excluded(k) => extend_one_byte(path(self.namespace, k)),
        };
        let max = match max {
            Bound::Unbounded => increment_last_byte(prefix_length(self.namespace)),
            Bound::Included(k) => extend_one_byte(path(self.namespace, k)),
            Bound::Excluded(k) => path(self.namespace, k),
        };

        let iter = store.scan(Some(&min), Some(&max), order).map(|(path, v)| {
            // strip the Map namespace from the path
            let (namespace, bytes) = split_one_key(&path)?;

            // the namespace returned by the store should always match the Map's
            debug_assert_eq!(namespace, self.namespace, "[Map]: namespace mispatch");

            // deserialize the key and value
            let key = K::deserialize(bytes)?;
            let data = from_json(&v)?;

            Ok((key, data))
        });

        Box::new(iter)
    }

    pub fn prefix<'a>(
        &self,
        store:       &dyn Storage,
        prefix:      &K::Prefix,
        start_after: Option<&K::Suffix>,
    ) -> Box<dyn Iterator<Item = anyhow::Result<(K::Suffix, T)>> + 'a> {
        todo!()
    }
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
/// Panics if any key's length exceeds u16::MAX (because we need to put the
/// length into 2 bytes)
fn path<K: MapKey>(namespace: &[u8], key: &K) -> Vec<u8> {
    let mut keys = key.serialize();

    // pop the last key. it doesn't need to be length-prefixed
    let last_key = keys.pop();
    let last_key_len = last_key.as_ref().map(|k| k.len()).unwrap_or(0);

    // compute the total length, so that we can allocate a Vec with the
    // necessary capacity at once, without having to reallocate
    let mut size = 2 + namespace.len() + last_key_len;
    for key in &keys {
        size += 2 + key.len();
    }

    // allocate the Vec and fill in the bytes
    let mut combined = Vec::with_capacity(size);
    combined.extend_from_slice(&encode_length(namespace));
    combined.extend_from_slice(namespace);
    for key in keys {
        combined.extend_from_slice(&encode_length(&key));
        combined.extend_from_slice(key.as_ref());
    }
    if let Some(k) = last_key {
        combined.extend_from_slice(k.as_ref());
    }
    combined
}

// TODO: replace with bound.map once stablized (seems like happening soon):
// https://github.com/rust-lang/rust/issues/86026
fn slice_bound<'a>(bound: Bound<&'a Vec<u8>>) -> Bound<&'a [u8]> {
    match bound {
        Bound::Excluded(bytes) => Bound::Excluded(bytes.as_slice()),
        Bound::Included(bytes) => Bound::Included(bytes.as_slice()),
        Bound::Unbounded => Bound::Unbounded,
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

fn prefix_length(bytes: &[u8]) -> Vec<u8> {
    let mut vec = Vec::with_capacity(bytes.len() + 2);
    vec.extend(encode_length(bytes));
    vec.extend_from_slice(bytes);
    vec
}

fn extend_one_byte(mut bytes: Vec<u8>) -> Vec<u8> {
    bytes.push(0);
    bytes
}

// NOTE: this doesn't work if the bytes are entirely 255.
// in practice, the input bytes is a length-prefixed Map namespace. for the
// bytes to be entirely 255, the namespace must be u16::MAX = 65535 byte long
// (so that the two prefixed length bytes are [255, 255]).
// we can prevent this by introducing a max length for the namespace.
// assert this max length at compile time when the user calls Map::new.
fn increment_last_byte(mut bytes: Vec<u8>) -> Vec<u8> {
    debug_assert!(bytes.iter().any(|x| *x != u8::MAX), "[Map]: Namespace is entirely 255");
    for byte in bytes.iter_mut().rev() {
        if *byte == u8::MAX {
            *byte = 0;
        } else {
            *byte += 1;
            break;
        }
    }
    bytes
}

fn split_one_key(bytes: &[u8]) -> anyhow::Result<(&[u8], &[u8])> {
    let (len_bytes, bytes) = bytes.split_at(2);
    let len = u16::from_be_bytes(len_bytes.try_into()?);
    Ok(bytes.split_at(len as usize))
}
