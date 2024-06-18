use {
    crate::{Borsh, Bound, Encoding, MapKey, Prefix, Proto, RawKey},
    borsh::BorshDeserialize,
    grug_types::{
        from_borsh_slice, from_proto_slice, nested_namespaces_with_key, Order, Record, StdError,
        StdResult, Storage,
    },
    prost::Message,
};

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

    pub fn range(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b>
    where
        T: 'b,
    {
        let de_fn = self.de_fn_kv;
        let pk_name = self.pk_name;

        let iter = self
            .inner
            .range_raw(store, min, max, order)
            .map(move |kv| (de_fn)(store, &pk_name, kv));

        Box::new(iter)
    }
}

// ----------------------------------- encoding -----------------------------------

macro_rules! recover_pk {
    (k $store:expr, $pk_namespace:expr, $kv:expr, $fn_des:ident) => {{
        let (key, pk_len) = $kv;
        let pk_len = $fn_des::<u32>(&pk_len)?;
        let offset = key.len() - pk_len as usize;
        let pk = &key[offset..];
        let empty_prefixes: &[&[u8]] = &[];

        let full_key = nested_namespaces_with_key(Some($pk_namespace), empty_prefixes, Some(&pk));

        let v = $store
            .read(&full_key)
            .ok_or_else(|| StdError::generic_err(format!("pk not found: {full_key:?}")))?;
        let v = $fn_des::<T>(&v)?;

        (pk.to_vec(), v)
    }};
}

macro_rules! generate_deserialize_fn {
    (v $fn_name:ident, $fn_des:ident where $($where:tt)+) => {
        fn $fn_name<$($where)+>(store: &dyn Storage, pk_namespace: &[u8], kv: Record) -> StdResult<(Vec<u8>, T)> {
            Ok(recover_pk!(k store, pk_namespace, kv, $fn_des))
        }
    };
    (kv $fn_name:ident, $fn_des:ident where $($where:tt)+) => {
        fn $fn_name<K: MapKey, $($where)+>(store: &dyn Storage, pk_namespace: &[u8], kv: Record) -> StdResult<(K::Output, T)> {
            let (k, v) = recover_pk!(k store, pk_namespace, kv, $fn_des);
            Ok((K::deserialize(&k)?, v))
        }
    };
}

generate_deserialize_fn!(v  borsh_deserialize_multi_v,  from_borsh_slice where T: BorshDeserialize);
generate_deserialize_fn!(kv borsh_deserialize_multi_kv, from_borsh_slice where T: BorshDeserialize);
generate_deserialize_fn!(v  proto_deserialize_multi_v,  from_proto_slice where T: Message + Default);
generate_deserialize_fn!(kv proto_deserialize_multi_kv, from_proto_slice where T: Message + Default);

macro_rules! index_prefix_encoding {
    ($encoding:tt, $fn_dv:ident, $fn_dkv:ident where $($where:tt)+  ) => {
        impl<'a, K, T> IndexPrefix<'a, K, T, $encoding>
        where
            K: MapKey,
            $($where)+,
        {
            pub fn with_deserialization_functions(
                top_name: &[u8],
                sub_names: &[RawKey],
                pk_name: &'a [u8],
            ) -> Self {
                Self {
                    inner: Prefix::<_,_, $encoding>::new(top_name, sub_names),
                    pk_name,
                    de_fn_kv: $fn_dkv::<K, _>,
                    de_fn_v: $fn_dv,
                }
            }
        }
    };
}

index_prefix_encoding!(Borsh, borsh_deserialize_multi_v, borsh_deserialize_multi_kv where T: BorshDeserialize);
index_prefix_encoding!(Proto, proto_deserialize_multi_v, proto_deserialize_multi_kv where T: Default + Message);
