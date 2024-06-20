use {
    crate::{Borsh, Bound, Codec, Index, Key, Map, Prefix, Set},
    grug_types::{Empty, Order, Record, StdResult, Storage},
};

// -------------------------------- multi index --------------------------------

/// An indexer that allows multiple records in the primary map to have the same
/// index value.
pub struct MultiIndex<'a, PK, IK, T, C: Codec<T> = Borsh> {
    indexer: fn(PK, &T) -> IK,
    index_set: Set<'a, (IK, PK)>,
    primary_map: Map<'a, PK, T, C>,
}

impl<'a, PK, IK, T, C: Codec<T>> MultiIndex<'a, PK, IK, T, C> {
    pub const fn new(
        indexer: fn(PK, &T) -> IK,
        pk_namespace: &'a str,
        idx_namespace: &'static str,
    ) -> Self {
        MultiIndex {
            indexer,
            index_set: Set::new(idx_namespace),
            primary_map: Map::new(pk_namespace),
        }
    }
}

impl<'a, PK, IK, T, C: Codec<T>> Index<PK, T> for MultiIndex<'a, PK, IK, T, C>
where
    PK: Key + Clone,
    IK: Key,
{
    fn save(&self, storage: &mut dyn Storage, pk: PK, data: &T) -> StdResult<()> {
        let idx = (self.indexer)(pk.clone(), data);
        self.index_set.insert(storage, (idx, pk))
    }

    fn remove(&self, storage: &mut dyn Storage, pk: PK, old_data: &T) {
        let idx = (self.indexer)(pk.clone(), old_data);
        self.index_set.remove(storage, (idx, pk))
    }
}

impl<'a, PK, IK, T, C: Codec<T>> MultiIndex<'a, PK, IK, T, C>
where
    PK: Key,
    IK: Key,
{
    /// Iterate records under a specific index value.
    pub fn of(&self, idx: IK) -> IndexPrefix<PK, T, C> {
        IndexPrefix {
            prefix: self.index_set.prefix(idx),
            primary_map: &self.primary_map,
        }
    }
}

// ---------------------------------- prefix -----------------------------------

pub struct IndexPrefix<'a, PK, T, C: Codec<T>> {
    prefix: Prefix<PK, Empty, Borsh>,
    primary_map: &'a Map<'a, PK, T, C>,
}

impl<'a, PK, T, C> IndexPrefix<'a, PK, T, C>
where
    PK: Key,
    C: Codec<T>,
{
    /// Iterate the raw primary keys and raw values under the given index value.
    pub fn range_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<PK>>,
        max: Option<Bound<PK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .prefix
            .keys_raw(storage, min, max, order)
            .map(|pk_raw| {
                // Load the data corresponding to the primary key from the
                // primary map.
                //
                // If the indexed map works correctly, the data should always exist,
                // so we can safely unwrap the `Option` here.
                let v_raw = self.primary_map.may_load_raw(storage, &pk_raw).unwrap();
                (pk_raw, v_raw)
            });

        Box::new(iter)
    }

    /// Iterate the primary keys and values under the given index value.
    pub fn range<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<PK>>,
        max: Option<Bound<PK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(PK::Output, T)>> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .prefix
            .keys_raw(storage, min, max, order)
            .map(|pk_raw| {
                let pk = PK::deserialize(&pk_raw)?;
                let v_raw = self.primary_map.load_raw(storage, &pk_raw)?;
                let v = C::decode(&v_raw)?;
                Ok((pk, v))
            });

        Box::new(iter)
    }

    /// Iterate the raw primary keys under the given index value.
    pub fn keys_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<PK>>,
        max: Option<Bound<PK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.prefix.keys_raw(storage, min, max, order)
    }

    /// Iterate the primary keys under the given index value.
    pub fn keys<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<PK>>,
        max: Option<Bound<PK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<PK::Output>> + 'b> {
        self.prefix.keys(storage, min, max, order)
    }

    /// Iterate the raw values under the given index value.
    pub fn values_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<PK>>,
        max: Option<Bound<PK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .prefix
            .keys_raw(storage, min, max, order)
            .map(|pk_raw| self.primary_map.load_raw(storage, &pk_raw).unwrap());

        Box::new(iter)
    }

    /// Iterate the values under the given index value.
    pub fn values<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<PK>>,
        max: Option<Bound<PK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<T>> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .prefix
            .keys_raw(storage, min, max, order)
            .map(|pk_raw| {
                let v_raw = self.primary_map.load_raw(storage, &pk_raw)?;
                C::decode(&v_raw)
            });

        Box::new(iter)
    }
}
