use {
    crate::{Borsh, Encoding, Index, IndexPrefix, Map, MapKey},
    grug_types::{StdResult, Storage},
    std::marker::PhantomData,
};

pub struct MultiIndex<'a, IK, T, PK, E: Encoding<T> = Borsh> {
    index: fn(&[u8], &T) -> IK,
    idx_namespace: &'a [u8],
    /// Use the Borsh encoding to encode the len of the pk.
    /// Handling also the case would require E: Encoding<u32>.
    idx_map: Map<'a, &'a [u8], u32, Borsh>,
    pk_namespace: &'a [u8],
    phantom_pk: PhantomData<PK>,
    phantom_e: PhantomData<E>,
}

impl<'a, IK, T, PK, E: Encoding<T>> Index<T> for MultiIndex<'a, IK, T, PK, E>
where
    IK: MapKey,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(pk, data).joined_extra_key(pk);
        let pk_len = pk.len() as u32;
        self.idx_map.save(store, &idx, &pk_len)
    }

    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = (self.index)(pk, old_data).joined_extra_key(pk);
        self.idx_map.remove(store, &idx);
        Ok(())
    }
}

impl<'a, IK, T, PK, E: Encoding<T>> MultiIndex<'a, IK, T, PK, E> {
    pub const fn new(
        idx_fn: fn(&[u8], &T) -> IK,
        pk_namespace: &'a str,
        idx_namespace: &'static str,
    ) -> Self {
        MultiIndex {
            index: idx_fn,
            idx_namespace: idx_namespace.as_bytes(),
            idx_map: Map::new(idx_namespace),
            pk_namespace: pk_namespace.as_bytes(),
            phantom_pk: PhantomData,
            phantom_e: PhantomData,
        }
    }
}

impl<'a, IK, T, PK, E: Encoding<T>> MultiIndex<'a, IK, T, PK, E>
where
    PK: MapKey,
    IK: MapKey,
{
    pub fn no_prefix(&self) -> IndexPrefix<PK, T, E, (IK, PK)> {
        IndexPrefix::with_deserialization_functions(self.idx_namespace, &[], self.pk_namespace)
    }

    pub fn prefix(&self, p: IK) -> IndexPrefix<PK, T, E, PK> {
        IndexPrefix::with_deserialization_functions(
            self.idx_namespace,
            &p.raw_keys(),
            self.pk_namespace,
        )
    }

    pub fn sub_prefix(&self, p: IK::Prefix) -> IndexPrefix<PK, T, E, (IK::Suffix, PK)> {
        IndexPrefix::with_deserialization_functions(
            self.idx_namespace,
            &p.raw_keys(),
            self.pk_namespace,
        )
    }
}
