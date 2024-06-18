use {
    crate::{Borsh, Index, IndexPrefix, Map, MapKey, Proto},
    borsh::BorshDeserialize,
    grug_types::{StdResult, Storage},
    prost::Message,
    std::marker::PhantomData,
};

pub struct MultiIndex<'a, IK, T, PK, E = Borsh> {
    index: fn(&[u8], &T) -> IK,
    idx_namespace: &'a [u8],
    idx_map: Map<'a, &'a [u8], u32>,
    pk_namespace: &'a [u8],
    phantom_pk: PhantomData<PK>,
    phantom_e: PhantomData<E>,
}

impl<'a, IK, T, PK> Index<T> for MultiIndex<'a, IK, T, PK>
where
    IK: MapKey,
{
    fn save(&self, store: &mut dyn Storage, pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(pk, data).joined_extra_key(pk);
        self.idx_map.save(store, &idx, &(pk.len() as u32))
    }

    fn remove(&self, store: &mut dyn Storage, pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = (self.index)(pk, old_data).joined_extra_key(pk);
        self.idx_map.remove(store, &idx);
        Ok(())
    }
}

impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK> {
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

// ----------------------------------- encoding -----------------------------------

macro_rules! multi_index_encoding {
    ($encoding:tt where $($where:tt)+) => {
        impl<'a, IK, T, PK> MultiIndex<'a, IK, T, PK, $encoding>
        where
            PK: MapKey,
            IK: MapKey,
            $($where)+
        {
            pub fn no_prefix(&self) -> IndexPrefix<PK, T, $encoding> {
                IndexPrefix::<_, _, $encoding>::with_deserialization_functions(
                    self.idx_namespace,
                    &[],
                    self.pk_namespace,
                )
            }

            pub fn prefix(&self, p: IK) -> IndexPrefix<PK, T, $encoding> {
                IndexPrefix::<_, _, $encoding>::with_deserialization_functions(
                    self.idx_namespace,
                    &p.raw_keys(),
                    self.pk_namespace,
                )
            }

            pub fn sub_prefix(&self, p: IK::Prefix) -> IndexPrefix<PK, T, $encoding> {
                IndexPrefix::<_, _, $encoding>::with_deserialization_functions(
                    self.idx_namespace,
                    &p.raw_keys(),
                    self.pk_namespace,
                )
            }
        }
    };
}

multi_index_encoding!(Borsh where T: BorshDeserialize);
multi_index_encoding!(Proto where T: Message + Default);
