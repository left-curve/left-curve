use {
    crate::{Borsh, Bound, Encoding, MapKey, Prefix, RawKey},
    grug_types::{
        from_borsh_slice, nested_namespaces_with_key, Order, Record, StdError, StdResult, Storage,
    },
};

pub struct IndexPrefix<'a, K, T, E: Encoding<T> = Borsh, B = Vec<u8>>
where
    K: MapKey,
{
    inner: Prefix<K, T, E, B>,
    pk_name: &'a [u8],
}

impl<'b, K, T, E, B> IndexPrefix<'b, K, T, E, B>
where
    B: MapKey,
    K: MapKey,
    E: Encoding<T>,
{
    pub fn range_raw(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<B>>,
        max: Option<Bound<B>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(Vec<u8>, T)>> + 'b>
    where
        T: 'b,
    {
        let pk_name = self.pk_name;

        let iter = self
            .inner
            .range_raw(store, min, max, order)
            .map(move |kv| des_v::<T, E>(store, pk_name, kv));

        Box::new(iter)
    }

    pub fn range(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<B>>,
        max: Option<Bound<B>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b>
    where
        T: 'b,
    {
        let pk_name = self.pk_name;

        let iter = self
            .inner
            .range_raw(store, min, max, order)
            .map(move |kv| des_kv::<T, K, E>(store, pk_name, kv));

        Box::new(iter)
    }

    pub fn keys_raw(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.inner.keys_raw(store, min, max, order)
    }

    pub fn keys(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<<K as MapKey>::Output>> + 'b> {
        self.inner.keys(store, min, max, order)
    }

    pub fn clear(
        &self,
        storage: &mut dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        limit: Option<usize>,
    ) {
        self.inner.clear(storage, min, max, limit)
    }

    pub fn is_empty(&self, storage: &dyn Storage) -> bool {
        self.inner
            .keys_raw(storage, None, None, Order::Ascending)
            .next()
            .is_none()
    }
}

impl<'a, K, T, E, B> IndexPrefix<'a, K, T, E, B>
where
    K: MapKey,
    E: Encoding<T>,
{
    pub fn with_deserialization_functions(
        top_name: &[u8],
        sub_names: &[RawKey],
        pk_name: &'a [u8],
    ) -> Self {
        Self {
            inner: Prefix::new(top_name, sub_names),
            pk_name,
        }
    }
}

macro_rules! recover_pk {
    ($store:expr, $pk_namespace:expr, $kv:expr) => {{
        let (key, pk_len) = $kv;
        let pk_len: u32 = from_borsh_slice(&pk_len)?;
        let offset = key.len() - pk_len as usize;
        let pk = &key[offset..];
        let empty_prefixes: &[&[u8]] = &[];

        let full_key = nested_namespaces_with_key(Some($pk_namespace), empty_prefixes, Some(&pk));

        let v = $store
            .read(&full_key)
            .ok_or_else(|| StdError::generic_err(format!("pk not found: {full_key:?}")))?;
        let v = E::decode(&v)?;

        (pk.to_vec(), v)
    }};
}

fn des_v<T, E: Encoding<T>>(
    store: &dyn Storage,
    pk_namespace: &[u8],
    kv: Record,
) -> StdResult<(Vec<u8>, T)> {
    Ok(recover_pk!(store, pk_namespace, kv))
}

fn des_kv<T, K: MapKey, E: Encoding<T>>(
    store: &dyn Storage,
    pk_namespace: &[u8],
    kv: Record,
) -> StdResult<(K::Output, T)> {
    let (pk, v) = recover_pk!(store, pk_namespace, kv);
    Ok((K::deserialize(&pk)?, v))
}
