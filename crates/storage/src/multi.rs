use {
    crate::{Borsh, Bound, Codec, Index, Key, Map, MultiIndexKey, Prefix, Set},
    grug_types::{Empty, Order, Record, StdResult, Storage},
};

// -------------------------------- multi index --------------------------------

/// An indexer that allows multiple records in the primary map to have the same
/// index value.
pub struct MultiIndex<'a, PK, IK, T, E: Codec<T> = Borsh>
where
    PK: MultiIndexKey,
    IK: MultiIndexKey,
{
    indexer: fn(&PK, &T) -> IK,
    index_set: Set<'a, (IK::MIPrefix, IK::MISuffix, PK::MIPrefix, PK::MISuffix)>,
    primary_map: Map<'a, PK, T, E>,
}

impl<'a, PK, IK, T, E: Codec<T>> MultiIndex<'a, PK, IK, T, E>
where
    PK: MultiIndexKey,
    IK: MultiIndexKey,
{
    pub const fn new(
        indexer: fn(&PK, &T) -> IK,
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

impl<'a, PK, IK, T, E: Codec<T>> Index<PK, T> for MultiIndex<'a, PK, IK, T, E>
where
    PK: MultiIndexKey,
    IK: MultiIndexKey,
{
    fn save(&self, storage: &mut dyn Storage, pk: PK, data: &T) -> StdResult<()> {
        let idx = (self.indexer)(&pk, data);
        // idx.
        self.index_set.insert(
            storage,
            (
                idx.index_prefix(),
                idx.index_suffix(),
                pk.index_prefix(),
                pk.index_suffix(),
            ),
        )
    }

    fn remove(&self, storage: &mut dyn Storage, pk: PK, old_data: &T) {
        let idx = (self.indexer)(&pk, old_data);
        self.index_set.remove(
            storage,
            (
                idx.index_prefix(),
                idx.index_suffix(),
                pk.index_prefix(),
                pk.index_suffix(),
            ),
        )
    }
}

impl<'a, PK, IK, T, E: Codec<T>> MultiIndex<'a, PK, IK, T, E>
where
    PK: MultiIndexKey,
    IK: MultiIndexKey,
{
    /// Iterate records under a specific index value.
    pub fn of(&self, idx: IK) -> IndexPrefix<(IK::MIPrefix, IK::MISuffix), PK, T, E> {
        // Create a tuple to have the correct len before keys
        let t = (idx.index_prefix(), idx.index_suffix());
        IndexPrefix {
            prefix: Prefix::new(self.index_set.namespace, &t.raw_keys()),
            primary_map: &self.primary_map,
            idx_ns: self.index_set.namespace.len(),
        }
    }

    /// Iterate records under a specific index prefix value.
    pub fn of_prefix(&self, idx: IK::MIPrefix) -> IndexPrefix<IK::MIPrefix, PK, T, E> {
        // IndexPrefix<T> What should be T?
        IndexPrefix {
            prefix: Prefix::new(self.index_set.namespace, &idx.raw_keys()),
            primary_map: &self.primary_map,
            idx_ns: self.index_set.namespace.len(),
        }
    }

    /// Iterate records under a specific index value and pk suffix.
    pub fn of_suffix(
        &self,
        // Should be better to have idx; (IK::IndexPrefix, IK::IndexSuffix, PK::IndexPrefix)
        idx: IK,
        suffix: PK::MIPrefix,
    ) -> IndexPrefix<IK::MISuffix, PK, T, E> {
        // IndexPrefix<T> What should be T?
        // Create a tuple to have the correct len before keys
        let t = (idx.index_prefix(), idx.index_suffix(), suffix);
        IndexPrefix {
            prefix: Prefix::new(self.index_set.namespace, &t.raw_keys()),
            primary_map: &self.primary_map,
            idx_ns: self.index_set.namespace.len(),
        }
    }
}

// ---------------------------------- prefix -----------------------------------

pub struct IndexPrefix<'a, RIK, PK, T, E: Codec<T>> {
    prefix: Prefix<RIK, Empty, Borsh>,
    primary_map: &'a Map<'a, PK, T, E>,
    idx_ns: usize,
}

impl<'a, RIK, PK, T, E> IndexPrefix<'a, RIK, PK, T, E>
where
    RIK: MultiIndexKey,
    PK: MultiIndexKey,
    E: Codec<T>,
{
    /// Iterate the raw primary keys and raw values under the given index value.
    pub fn range_raw<'b>(
        &'b self,
        storage: &'b dyn Storage,
        min: Option<Bound<RIK>>,
        max: Option<Bound<RIK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .prefix
            .keys_raw_no_trimmer(storage, min, max, order)
            .map(|pk_raw| {
                // Load the data corresponding to the primary key from the
                // primary map.
                //
                // If the indexed map works correctly, the data should always exist,
                // so we can safely unwrap the `Option` here.
                let pk_raw = self.trim_key(&pk_raw);
                let v_raw = self.primary_map.may_load_raw(storage, &pk_raw).unwrap();
                (pk_raw, v_raw)
            });

        Box::new(iter)
    }

    /// Iterate the primary keys and values under the given index value.
    pub fn range<'b>(
        &'b self,
        storage: &'b dyn Storage,
        min: Option<Bound<RIK>>,
        max: Option<Bound<RIK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<(PK::Output, T)>> + 'b>
    where
        'a: 'b,
    {
        let iter = self
            .prefix
            .keys_raw_no_trimmer(storage, min, max, order)
            .map(|pk_raw| {
                let pk_raw = self.trim_key(&pk_raw);
                let pk = PK::deserialize_from_index(&pk_raw)?;
                let v_raw = self
                    .primary_map
                    .load_raw(storage, PK::adjust_from_index(&pk_raw))?;
                let v = E::decode(&v_raw)?;
                Ok((pk, v))
            });

        Box::new(iter)
    }

    /// Iterate the raw primary keys under the given index value.
    pub fn keys_raw<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<RIK>>,
        max: Option<Bound<RIK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b> {
        self.prefix.keys_raw(storage, min, max, order)
    }

    /// Iterate the primary keys under the given index value.
    pub fn keys<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<RIK>>,
        max: Option<Bound<RIK>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<RIK::Output>> + 'b> {
        self.prefix.keys(storage, min, max, order)
    }


    fn trim_key(&self, key: &[u8]) -> Vec<u8> {
        let mut key = &key[self.idx_ns + 2..];

        // We trim the IK::Suffix and PK::Prefix.
        for _ in 0..2 {
            let (len, rest) = key.split_at(2);

            let a = u16::from_be_bytes([len[0], len[1]]);

            key = &rest[a as usize..]
        }

        key.to_vec()

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
