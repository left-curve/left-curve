use {
    crate::{Borsh, Bound, Encoding, Index, Map, MapKey, Prefix, Proto},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{Order, Record, StdError, StdResult, Storage},
    prost::Message,
};

pub trait IndexList<T> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &'_ dyn Index<T>> + '_>;
}

/// `IndexedMap` works like a `Map` but has a secondary index
pub struct IndexedMap<'a, K, T, I, E: Encoding = Borsh> {
    pk_namespace: &'a [u8],
    primary: Map<'a, K, T, E>,
    /// This is meant to be read directly to get the proper types, like:
    /// map.idx.owner.items(...)
    pub idx: I,
}

impl<'a, K, T, I> IndexedMap<'a, K, T, I>
where
    K: MapKey,
    I: IndexList<T>,
{
    pub const fn new(pk_namespace: &'static str, indexes: I) -> Self {
        IndexedMap {
            pk_namespace: pk_namespace.as_bytes(),
            primary: Map::new(pk_namespace),
            idx: indexes,
        }
    }

    pub fn has(&self, storage: &dyn Storage, k: K) -> bool {
        self.primary.has(storage, k)
    }
}

impl<'a, K, T, I, E> IndexedMap<'a, K, T, I, E>
where
    K: MapKey,
    I: IndexList<T>,
    E: Encoding,
{
    pub fn is_empty(&self, storage: &dyn Storage) -> bool {
        self.no_prefix()
            .keys_raw(storage, None, None, Order::Ascending)
            .next()
            .is_none()
    }

    fn no_prefix(&self) -> Prefix<K, T, E> {
        Prefix::new(self.pk_namespace, &[])
    }

    pub fn prefix(&self, prefix: K::Prefix) -> Prefix<K::Suffix, T, E> {
        Prefix::new(self.pk_namespace, &prefix.raw_keys())
    }

    pub fn range_raw<'b>(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'b>
    where
        T: 'b,
    {
        self.no_prefix().range_raw(store, min, max, order)
    }

    pub fn keys_raw<'b>(
        &self,
        store: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = Vec<u8>> + 'b>
    where
        T: 'b,
    {
        self.no_prefix().keys_raw(store, min, max, order)
    }

    pub fn keys<'b>(
        &self,
        storage: &'b dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        order: Order,
    ) -> Box<dyn Iterator<Item = StdResult<K::Output>> + 'b> {
        self.no_prefix().keys(storage, min, max, order)
    }

    pub fn clear(
        &self,
        storage: &mut dyn Storage,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        limit: Option<usize>,
    ) {
        self.no_prefix().clear(storage, min, max, limit)
    }
}

// ----------------------------------- encoding -----------------------------------

macro_rules! index_map_encoding {
    ($encoding:tt where $($where:tt)+) => {
        impl<'a, K, T, I> IndexedMap<'a, K, T, I, $encoding>
        where
            K: MapKey,
            $($where)+
        {
            pub fn load(&self, storage: &dyn Storage, key: K) -> StdResult<T> {
                self.primary.load(storage, key)
            }

            pub fn may_load(&self, storage: &dyn Storage, key: K) -> StdResult<Option<T>> {
                self.primary.may_load(storage, key)
            }
        }

        impl<'a, K, T, I> IndexedMap<'a, K, T, I, $encoding>
        where
            K: MapKey,
            I: IndexList<T>,
            $($where)+
        {
            pub fn range<'b>(
                &self,
                storage: &'b dyn Storage,
                min: Option<Bound<K>>,
                max: Option<Bound<K>>,
                order: Order,
            ) -> Box<dyn Iterator<Item = StdResult<(K::Output, T)>> + 'b> {
                self.no_prefix().range(storage, min, max, order)
            }
        }

        impl<'a, K, T, I> IndexedMap<'a, K, T, I, $encoding>
        where
            K: MapKey + Clone,
            I: IndexList<T>,
            $($where)+
        {
            pub fn save(&'a self, storage: &mut dyn Storage, key: K, data: &T) -> StdResult<()> {
                let old_data = self.may_load(storage, key.clone())?;
                self.replace(storage, key, Some(data), old_data.as_ref())
            }

            pub fn remove(&'a self, storage: &mut dyn Storage, key: K) -> StdResult<()> {
                let old_data = self.may_load(storage, key.clone())?;
                self.replace(storage, key, None, old_data.as_ref())
            }

            pub fn replace(
                &'a self,
                storage: &mut dyn Storage,
                key: K,
                data: Option<&T>,
                old_data: Option<&T>,
            ) -> StdResult<()> {
                // this is the key *relative* to the primary map namespace
                let pk = key.serialize();
                if let Some(old) = old_data {
                    for index in self.idx.get_indexes() {
                        index.remove(storage, &pk, old)?;
                    }
                }
                if let Some(updated) = data {
                    for index in self.idx.get_indexes() {
                        index.save(storage, &pk, updated)?;
                    }
                    self.primary.save(storage, key, updated)?;
                } else {
                    self.primary.remove(storage, key);
                }
                Ok(())
            }
        }

        impl<'a, K, T, I> IndexedMap<'a, K, T, I, $encoding>
        where
            K: MapKey + Clone,
            I: IndexList<T>,
            $($where)+,
            T: Clone
        {
            pub fn update<A, Err>(
                &'a self,
                storage: &mut dyn Storage,
                key: K,
                action: A,
            ) -> Result<T, Err>
            where
                A: FnOnce(Option<T>) -> Result<T, Err>,
                Err: From<StdError>,
            {
                let input = self.may_load(storage, key.clone())?;
                let old_val = input.clone();
                let output = action(input)?;
                self.replace(storage, key, Some(&output), old_val.as_ref())?;
                Ok(output)
            }
        }
    };
}

index_map_encoding!(Borsh where T: BorshSerialize + BorshDeserialize);
index_map_encoding!(Proto where T: Message + Default);
