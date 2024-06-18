use {
    crate::{Borsh, Encoding, Index, Map, MapKey},
    grug_types::{StdError, StdResult, Storage},
    std::marker::PhantomData,
};
pub struct UniqueIndex<'a, IK, T, PK, E: Encoding<T> = Borsh> {
    index: fn(&T) -> IK,
    idx_map: Map<'a, IK, T, E>,
    _idx_namespace: &'a [u8],
    phantom_pk: PhantomData<PK>,
    phantom_e: PhantomData<E>,
}

impl<'a, IK, T, PK, E> UniqueIndex<'a, IK, T, PK, E>
where
    E: Encoding<T>,
{
    pub const fn new(idx_fn: fn(&T) -> IK, idx_namespace: &'static str) -> Self {
        UniqueIndex {
            index: idx_fn,
            idx_map: Map::new(idx_namespace),
            _idx_namespace: idx_namespace.as_bytes(),
            phantom_pk: PhantomData,
            phantom_e: PhantomData,
        }
    }
}

impl<'a, IK, T, PK, E> Index<T> for UniqueIndex<'a, IK, T, PK, E>
where
    IK: MapKey,
    E: Encoding<T>,
    T: Clone,
{
    fn save(&self, store: &mut dyn Storage, _pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(data);
        self.idx_map
            .update(store, idx, |existing| -> StdResult<_> {
                match existing {
                    Some(_) => Err(StdError::generic_err("Violates unique constraint on index")),
                    None => Ok(Some(data.clone())),
                }
            })?;
        Ok(())
    }

    fn remove(&self, store: &mut dyn Storage, _pk: &[u8], old_data: &T) -> StdResult<()> {
        let idx = (self.index)(old_data);
        self.idx_map.remove(store, idx);
        Ok(())
    }
}

impl<'a, IK, T, PK, E> UniqueIndex<'a, IK, T, PK, E>
where
    IK: MapKey,
    E: Encoding<T>,
{
    pub fn map(&self) -> &Map<'a, IK, T, E> {
        &self.idx_map
    }
}
