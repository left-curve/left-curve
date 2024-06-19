use {
    crate::{Borsh, Encoding, Index, Map, MapKey},
    grug_types::{StdError, StdResult, Storage},
    std::{marker::PhantomData, ops::Deref},
};

pub struct UniqueIndex<'a, IK, T, PK, E: Encoding<T> = Borsh> {
    index: fn(&T) -> IK,
    idx_map: Map<'a, IK, T, E>,
    _idx_namespace: &'a [u8],
    phantom_pk: PhantomData<PK>,
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

impl<'a, IK, T, PK, E> Deref for UniqueIndex<'a, IK, T, PK, E>
where
    E: Encoding<T>,
{
    type Target = Map<'a, IK, T, E>;

    fn deref(&self) -> &Self::Target {
        &self.idx_map
    }
}
