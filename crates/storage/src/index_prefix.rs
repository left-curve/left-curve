use borsh::BorshDeserialize;
use grug_types::{
    from_borsh_slice, nested_namespaces_with_key, Order, Record, StdError, StdResult, Storage,
};

use crate::{Borsh, Bound, Encoding, MapKey, Prefix, RawKey};

pub struct IndexPrefix<'a, K, T, E: Encoding = Borsh>
where
    K: MapKey,
{
    inner: Prefix<K, T, E>,
    pk_name: &'a [u8],
    de_fn_kv: DeserializeKvFn<K, T>,
    de_fn_v: DeserializeVFn<T>,
}

type DeserializeVFn<T> = fn(&dyn Storage, &[u8], Record) -> StdResult<(Vec<u8>, T)>;
type DeserializeKvFn<K, T> =
    fn(&dyn Storage, &[u8], Record) -> StdResult<(<K as MapKey>::Output, T)>;

impl<'b, K, T, E> IndexPrefix<'b, K, T, E>
where
    K: MapKey,
    E: Encoding,
{
    pub fn range_raw(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(Vec<u8>, T)>> + 'b>
    where
        T: 'b,
    {
        let de_fn = self.de_fn_v;
        let pk_name = self.pk_name;

        let iter = self
            .inner
            .range_raw(store, min, max, order)
            .map(move |kv| (de_fn)(store, &pk_name, kv));

        Box::new(iter)

        // Cosmwasm impl

        // let mapped = range_with_prefix(
        //     store,
        //     &self.inner.storage_prefix,
        //     min.map(|b| b.to_raw_bound()),
        //     max.map(|b| b.to_raw_bound()),
        //     order,
        // )
        // .map(move |kv| (de_fn)(store, &pk_name, kv));
        // Box::new(mapped)
    }
}

// ----------------------------------- encoding -----------------------------------

fn deserialize_multi_v<T: BorshDeserialize>(
    store: &dyn Storage,
    pk_namespace: &[u8],
    kv: Record,
) -> StdResult<(Vec<u8>, T)> {
    let (key, pk_len) = kv;

    // Deserialize pk_len
    let pk_len = from_borsh_slice::<u32>(&pk_len)?;

    // Recover pk from last part of k
    let offset = key.len() - pk_len as usize;
    let pk = &key[offset..];

    let empty_prefixes: &[&[u8]] = &[];

    let full_key = nested_namespaces_with_key(Some(pk_namespace), empty_prefixes, Some(&pk));

    let v = store
        .read(&full_key)
        .ok_or_else(|| StdError::generic_err(format!("pk not found: {full_key:?}")))?;
    let v = from_borsh_slice::<T>(&v)?;

    Ok((pk.to_vec(), v))
}

fn deserialize_multi_kv<K: MapKey, T: BorshDeserialize>(
    store: &dyn Storage,
    pk_namespace: &[u8],
    kv: Record,
) -> StdResult<(K::Output, T)> {
    let (key, pk_len) = kv;

    // Deserialize pk_len
    let pk_len = from_borsh_slice::<u32>(&pk_len)?;

    // Recover pk from last part of k
    let offset = key.len() - pk_len as usize;
    let pk = &key[offset..];

    let full_key = nested_namespaces_with_key(Some(pk_namespace), &[&[]], Some(&pk));

    let v = store
        .read(&full_key)
        .ok_or_else(|| StdError::generic_err(format!("pk not found: {full_key:?}")))?;
    let v = from_borsh_slice::<T>(&v)?;

    Ok((K::deserialize(&pk)?, v))
}

impl<'a, K, T> IndexPrefix<'a, K, T, Borsh>
where
    K: MapKey,
    T: BorshDeserialize,
{
    pub fn with_deserialization_functions(
        top_name: &[u8],
        sub_names: &[RawKey],
        pk_name: &'a [u8],
    ) -> Self {
        Self {
            inner: Prefix::new(top_name, sub_names),
            pk_name,
            de_fn_kv: deserialize_multi_kv::<K, _>,
            de_fn_v: deserialize_multi_v,
        }
    }
}
