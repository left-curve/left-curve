use {
    super::Indexer,
    crate::{Borsh, Codec, Index, Map, PrimaryKey, Raw},
    grug_types::{Bound, Order, StdError, StdResult, Storage},
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
pub struct UniqueIndex<'a, PK, IK, T, C = Borsh>
where
    PK: PrimaryKey,
    IK: PrimaryKey + Clone,
    C: Codec<T>,
{
    /// A function that takes a key-value pair, and returns the index key(s) it
    /// should be indexed at.
    indexer: Indexer<PK, IK, T>,
    // Index => _raw_ primary key
    index_map: Map<'a, IK, Vec<u8>, Raw>,
    // Primary key => data
    primary_map: Map<'a, PK, T, C>,
}

impl<'a, PK, IK, T, C> UniqueIndex<'a, PK, IK, T, C>
where
    PK: PrimaryKey,
    IK: PrimaryKey + Clone,
    C: Codec<T>,
{
    /// Note: The developer must make sure that `idx_namespace` is not the same
    /// as the primary map namespace.
    pub const fn new(
        indexer: fn(&PK, &T) -> IK,
        pk_namespace: &'static str,
        idx_namespace: &'static str,
    ) -> Self {
        UniqueIndex {
            indexer: Indexer::Single(indexer),
            index_map: Map::new(idx_namespace),
            primary_map: Map::new(pk_namespace),
        }
    }

    /// Create a `UniqueIndex` with an indexer that returns multiple index keys
    /// per record.
    pub const fn new2(
        indexer: fn(&PK, &T) -> Vec<IK>,
        pk_namespace: &'static str,
        idx_namespace: &'static str,
    ) -> Self {
        UniqueIndex {
            indexer: Indexer::Multi(indexer),
            index_map: Map::new(idx_namespace),
            primary_map: Map::new(pk_namespace),
        }
    }

    pub fn is_empty(&self, storage: &dyn Storage) -> bool {
        self.index_map.is_empty(storage)
    }

    /// Given an index value, which may or may not exist, load the corresponding
    /// key.
    pub fn may_load_key(&self, storage: &dyn Storage, idx: IK) -> StdResult<Option<PK::Output>> {
        self.index_map
            .may_load_raw(storage, &idx.joined_key())
            .map(|pk_raw| PK::from_slice(&pk_raw))
            .transpose()
    }

    /// Given an index value, load the corresponding key.
    pub fn load_key(&self, storage: &dyn Storage, idx: IK) -> StdResult<PK::Output> {
        self.index_map
            .load_raw(storage, &idx.joined_key())
            .and_then(|pk_raw| PK::from_slice(&pk_raw))
    }

    /// Given an index value, which may or may not exist, load the corresponding
    /// value.
    pub fn may_load_value(&self, storage: &dyn Storage, idx: IK) -> StdResult<Option<T>> {
        self.index_map
            .may_load_raw(storage, &idx.joined_key())
            .map(|pk_raw| self.primary_map.may_load_raw(storage, &pk_raw).unwrap())
            .map(|v_raw| C::decode(&v_raw))
            .transpose()
    }

    /// Given an index value, load the corresponding value.
    pub fn load_value(&self, storage: &dyn Storage, idx: IK) -> StdResult<T> {
        self.index_map
            .load_raw(storage, &idx.joined_key())
            .map(|pk_raw| self.primary_map.may_load_raw(storage, &pk_raw).unwrap())
            .and_then(|v_raw| C::decode(&v_raw))
    }

    /// Given an index value, which may or may not exist, load the corresponding
    /// key and value.
    pub fn may_load(&self, storage: &dyn Storage, idx: IK) -> StdResult<Option<(PK::Output, T)>> {
        self.index_map
            .may_load_raw(storage, &idx.joined_key())
            .map(|pk_raw| {
                let v_raw = self.primary_map.may_load_raw(storage, &pk_raw).unwrap();
                (pk_raw, v_raw)
            })
            .map(|(pk_raw, v_raw)| {
                let pk = PK::from_slice(&pk_raw)?;
                let v = C::decode(&v_raw)?;
                Ok((pk, v))
            })
            .transpose()
    }

    /// Given an index value, load the corresponding primary key and value.
    pub fn load(&self, storage: &dyn Storage, idx: IK) -> StdResult<(PK::Output, T)> {
        self.index_map
            .load_raw(storage, &idx.joined_key())
            .map(|pk_raw| {
                let v_raw = self.primary_map.may_load_raw(storage, &pk_raw).unwrap();
                (pk_raw, v_raw)
            })
            .and_then(|(pk_raw, v_raw)| {
                let pk = PK::from_slice(&pk_raw)?;
                let v = C::decode(&v_raw)?;
                Ok((pk, v))
            })
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
                let v_raw = self.primary_map.may_load_raw(storage, &pk_raw).unwrap();
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
    ) -> Box<dyn Iterator<Item = StdResult<(IK::Output, PK::Output, T)>> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .index_map
            .range_raw(storage, min, max, order)
            .map(|(ik_raw, pk_raw)| {
                let ik = IK::from_slice(&ik_raw)?;
                let pk = PK::from_slice(&pk_raw)?;
                let v_raw = self.primary_map.may_load_raw(storage, &pk_raw).unwrap();
                let v = C::decode(&v_raw)?;
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
    ) -> Box<dyn Iterator<Item = StdResult<(IK::Output, PK::Output)>> + 'b> {
        let iter = self
            .index_map
            .range_raw(storage, min, max, order)
            .map(|(ik_raw, pk_raw)| {
                let ik = IK::from_slice(&ik_raw)?;
                let pk = PK::from_slice(&pk_raw)?;
                Ok((ik, pk))
            });

        Box::new(iter)
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
                let v_raw = self.primary_map.may_load_raw(storage, &pk_raw).unwrap();
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
                let v_raw = self.primary_map.may_load_raw(storage, &pk_raw).unwrap();
                let v = C::decode(&v_raw)?;
                Ok((ik, v))
            });

        Box::new(iter)
    }
}

impl<PK, IK, T, C> Index<PK, T> for UniqueIndex<'_, PK, IK, T, C>
where
    PK: PrimaryKey,
    IK: PrimaryKey + Clone,
    C: Codec<T>,
{
    fn save(&self, storage: &mut dyn Storage, pk: PK, data: &T) -> StdResult<()> {
        let raw_pk = pk.joined_key();
        match &self.indexer {
            Indexer::Single(f) => {
                let idx = f(&pk, data);
                if self.index_map.has(storage, idx.clone()) {
                    return Err(StdError::duplicate_data::<IK>());
                }
                self.index_map.save(storage, idx, &raw_pk)
            },
            Indexer::Multi(f) => {
                for idx in f(&pk, data) {
                    if self.index_map.has(storage, idx.clone()) {
                        return Err(StdError::duplicate_data::<IK>());
                    }
                    self.index_map.save(storage, idx, &raw_pk)?;
                }
                Ok(())
            },
        }
    }

    fn remove(&self, storage: &mut dyn Storage, pk: PK, old_data: &T) {
        match &self.indexer {
            Indexer::Single(f) => {
                self.index_map.remove(storage, f(&pk, old_data));
            },
            Indexer::Multi(f) => {
                for idx in f(&pk, old_data) {
                    self.index_map.remove(storage, idx);
                }
            },
        }
    }

    fn clear_all(&self, storage: &mut dyn Storage) {
        self.index_map.clear(storage, None, None)
    }
}
