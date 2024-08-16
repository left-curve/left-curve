use {
    crate::{Borsh, Bound, Codec, Index, Key, Map},
    grug_types::{Order, StdError, StdResult, Storage},
};

/// An indexer that ensures that indexes are unique, meaning no two records in
/// the primary map may have the same index.
///
/// Internally, a `UniqueIndex` is a wrapper around a `Map` that maps index keys
/// to values. Essentially, when a key-value pair is stored in the `Map`, the
/// value is stored twice:
///
/// - In the primary map: (pk_namespace, pk) => value
/// - in the index map: (idx_namespace, ik) => value
pub struct UniqueIndex<'a, PK, IK, T, PC = Borsh, IC = Borsh>
where
    PK: Key + Clone,
    IK: Key + Clone,
    PC: Codec<T>,
    IC: Codec<PK>,
{
    /// A function that takes a piece of data, and return the index key it
    /// should be indexed at.
    indexer: fn(&PK, &T) -> IK,
    /// Primary key by the index key.
    index_map: Map<'a, IK, PK, IC>,
    /// Data indexed by primary key.
    primary_map: Map<'a, PK, T, PC>,
}

impl<'a, PK, IK, T, PC, IC> UniqueIndex<'a, PK, IK, T, PC, IC>
where
    PK: Key + Clone,
    IK: Key + Clone,
    PC: Codec<T>,
    IC: Codec<PK>,
{
    /// Note: The developer must make sure that `idx_namespace` is not the same
    /// as the primary map namespace.
    pub const fn new(
        indexer: fn(&PK, &T) -> IK,
        pk_namespace: &'static str,
        idx_namespace: &'static str,
    ) -> Self {
        UniqueIndex {
            indexer,
            index_map: Map::new(idx_namespace),
            primary_map: Map::new(pk_namespace),
        }
    }

    /// Given an index value, load the corresponding key.
    pub fn load_key(&self, storage: &dyn Storage, idx: IK) -> StdResult<PK> {
        self.index_map.load(storage, idx)
    }

    /// Given an index value, load the corresponding value.
    pub fn load_value(&self, storage: &dyn Storage, idx: IK) -> StdResult<T> {
        let pk = self.index_map.load(storage, idx)?;

        self.primary_map.load(storage, pk)
    }

    /// Given an index value, load the corresponding primary key and value.
    pub fn load(&self, storage: &dyn Storage, idx: IK) -> StdResult<(PK, T)> {
        let pk = self.index_map.load(storage, idx)?;
        let v = self.primary_map.load(storage, pk.clone())?;

        Ok((pk, v))
    }

    /// Iterate all {index, primary key, value} tuples within a bound of indexes,
    /// without deserialization.
    pub fn range_raw<'b>(
        &'b self,
        storage: &'b dyn Storage,
        min: Option<Bound<IK>>,
        max: Option<Bound<IK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>, Vec<u8>)> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .index_map
            .range_raw(storage, min, max, order)
            .map(|(ik_raw, pk_raw)| {
                let v_raw = self.primary_map.load_raw(storage, &pk_raw).unwrap();
                (ik_raw, pk_raw, v_raw)
            });

        Box::new(iter)
    }

    /// Iterate all {index, primary key, value} tuples within a bound of indexes.
    pub fn range<'b>(
        &'b self,
        storage: &'b dyn Storage,
        min: Option<Bound<IK>>,
        max: Option<Bound<IK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(IK::Output, PK, T)>> + 'b>
    where
        'a: 'b,
    {
        let iter = self.index_map.range(storage, min, max, order).map(|ik_pk| {
            let (ik, pk) = ik_pk?;
            let v = self.primary_map.load(storage, pk.clone())?;
            Ok((ik, pk, v))
        });

        Box::new(iter)
    }

    /// Iterate all {index, primary key} tuples within a bound of indexes,
    /// without deserialization.
    pub fn keys_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<IK>>,
        max: Option<Bound<IK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'b> {
        self.index_map.range_raw(storage, min, max, order)
    }

    /// Iterate all {index, primary key} tuples within a bound of indexes.
    pub fn keys<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<IK>>,
        max: Option<Bound<IK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(IK::Output, PK)>> + 'b> {
        self.index_map.range(storage, min, max, order)
    }

    /// Iterate all {index, value} tuples within a bound of indexes, without
    /// deserialization.
    pub fn values_raw<'b>(
        &'b self,
        storage: &'b dyn Storage,
        min: Option<Bound<IK>>,
        max: Option<Bound<IK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .index_map
            .range_raw(storage, min, max, order)
            .map(|(ik_raw, pk_raw)| {
                let v_raw = self.primary_map.load_raw(storage, &pk_raw).unwrap();
                (ik_raw, v_raw)
            });

        Box::new(iter)
    }

    /// Iterate all {index, value} tuples within a bound of indexes.
    pub fn values<'b>(
        &'b self,
        storage: &'b dyn Storage,
        min: Option<Bound<IK>>,
        max: Option<Bound<IK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(IK::Output, T)>> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .index_map
            .range_raw(storage, min, max, order)
            .map(|(ik_raw, pk_raw)| {
                let ik = IK::from_slice(&ik_raw)?;
                let v_raw = self.primary_map.load_raw(storage, &pk_raw).unwrap();
                let v = PC::decode(&v_raw)?;
                Ok((ik, v))
            });

        Box::new(iter)
    }
}

impl<'a, PK, IK, T, PC, IC> Index<PK, T> for UniqueIndex<'a, PK, IK, T, PC, IC>
where
    PK: Key + Clone,
    IK: Key + Clone,
    PC: Codec<T>,
    IC: Codec<PK>,
{
    fn save(&self, storage: &mut dyn Storage, pk: PK, data: &T) -> StdResult<()> {
        let idx = (self.indexer)(&pk, data);

        // Ensure that indexes are unique.
        if self.index_map.has(storage, idx.clone()) {
            return Err(StdError::duplicate_data::<IK>(&idx.joined_key()));
        }

        self.index_map.save(storage, idx, &pk)
    }

    fn remove(&self, storage: &mut dyn Storage, pk: PK, old_data: &T) {
        let idx = (self.indexer)(&pk, old_data);
        self.index_map.remove(storage, idx);
    }
}
