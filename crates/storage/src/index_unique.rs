use {
    crate::{Borsh, Encoding, Index, Map, MapKey},
    grug_types::{StdError, StdResult, Storage},
    std::ops::Deref,
};

pub struct UniqueIndex<'a, IK, T, E: Encoding<T> = Borsh> {
    /// A function that takes a piece of data, and return the index key it
    /// should be indexed at.
    index: fn(&T) -> IK,
    /// Data indexed by the index key.
    ///
    /// Essentially, in an index map, each piece of data is stored twice.
    /// Once in the primary map under the primary key, then again here under the
    /// index key.
    idx_map: Map<'a, IK, T, E>,
}

impl<'a, IK, T, E> UniqueIndex<'a, IK, T, E>
where
    E: Encoding<T>,
{
    /// Note: The developer must make sure that `idx_namespace` is not the same
    /// as the primary map namespace. It isn't possible for the `UniqueIndex` to
    /// assert this at compile time.
    pub const fn new(idx_fn: fn(&T) -> IK, idx_namespace: &'static str) -> Self {
        UniqueIndex {
            index: idx_fn,
            idx_map: Map::new(idx_namespace),
        }
    }
}

// Since the `UniqueIndex` is essentially a wrapper of a `Map` (`self.idx_map`),
// we let it dereference to the inner map. This way, users are able to directly
// call methods on the inner map, such as `range`, `prefix`, etc.
impl<'a, IK, T, E> Deref for UniqueIndex<'a, IK, T, E>
where
    E: Encoding<T>,
{
    type Target = Map<'a, IK, T, E>;

    fn deref(&self) -> &Self::Target {
        &self.idx_map
    }
}

impl<'a, IK, T, E> Index<T> for UniqueIndex<'a, IK, T, E>
where
    IK: MapKey + Clone,
    E: Encoding<T>,
    T: Clone,
{
    fn save(&self, storage: &mut dyn Storage, _pk: &[u8], data: &T) -> StdResult<()> {
        let idx = (self.index)(data);

        // Enforce that indexes are unique: two records in the primary map
        // cannot have the same index.
        if self.idx_map.has(storage, idx.clone()) {
            // TODO: create a `StdError::DuplicateData` for this?
            return Err(StdError::generic_err("Violates unique constraint on index"));
        }

        self.idx_map.save(storage, idx, data)
    }

    fn remove(&self, storage: &mut dyn Storage, _pk: &[u8], old_data: &T) {
        let idx = (self.index)(old_data);
        self.idx_map.remove(storage, idx);
    }
}
