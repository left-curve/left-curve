use {
    crate::{range_bounds, Codec, Prefix, PrimaryKey},
    grug_types::{
        nested_namespaces_with_key, trim, Addr, Binary, Bound, QuerierWrapper, Query,
        QueryWasmRawRequest, QueryWasmScanRequest, StdError, StdResult, Storage,
    },
    std::{borrow::Cow, marker::PhantomData},
};

pub struct Path<'a, T, C> {
    storage_key: Cow<'a, [u8]>,
    data: PhantomData<T>,
    codec: PhantomData<C>,
}

impl<'a, T, C> Path<'a, T, C>
where
    C: Codec<T>,
{
    pub fn new(namespace: &[u8], prefixes: &[Cow<[u8]>], maybe_key: Option<&Cow<[u8]>>) -> Self {
        Self {
            storage_key: Cow::Owned(nested_namespaces_with_key(
                Some(namespace),
                prefixes,
                maybe_key,
            )),
            data: PhantomData,
            codec: PhantomData,
        }
    }

    pub const fn from_raw(storage_key: &'a [u8]) -> Self {
        Self {
            storage_key: Cow::Borrowed(storage_key),
            data: PhantomData,
            codec: PhantomData,
        }
    }

    #[inline]
    pub fn storage_key(&self) -> &[u8] {
        self.storage_key.as_ref()
    }

    pub fn exists(&self, storage: &dyn Storage) -> bool {
        storage.read(self.storage_key()).is_some()
    }

    pub fn may_load_raw(&self, storage: &dyn Storage) -> Option<Vec<u8>> {
        storage.read(self.storage_key())
    }

    pub fn may_load(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        storage
            .read(self.storage_key())
            .map(|val| C::decode(&val))
            .transpose()
    }

    pub fn load_raw(&self, storage: &dyn Storage) -> StdResult<Vec<u8>> {
        storage
            .read(self.storage_key())
            .ok_or_else(|| StdError::data_not_found::<T>(self.storage_key()))
    }

    pub fn load(&self, storage: &dyn Storage) -> StdResult<T> {
        storage
            .read(self.storage_key())
            .ok_or_else(|| StdError::data_not_found::<T>(self.storage_key()))
            .and_then(|val| C::decode(&val))
    }

    pub fn may_take_raw(&self, storage: &mut dyn Storage) -> Option<Vec<u8>> {
        let maybe_data = self.may_load_raw(storage);

        if maybe_data.is_some() {
            self.remove(storage);
        }

        maybe_data
    }

    pub fn may_take(&self, storage: &mut dyn Storage) -> StdResult<Option<T>> {
        let maybe_data = self.may_load(storage)?;

        if maybe_data.is_some() {
            self.remove(storage);
        }

        Ok(maybe_data)
    }

    pub fn take_raw(&self, storage: &mut dyn Storage) -> StdResult<Vec<u8>> {
        let data = self.load_raw(storage)?;

        self.remove(storage);

        Ok(data)
    }

    pub fn take(&self, storage: &mut dyn Storage) -> StdResult<T> {
        let data = self.load(storage)?;

        self.remove(storage);

        Ok(data)
    }

    pub fn save_raw(&self, storage: &mut dyn Storage, data_raw: &[u8]) {
        storage.write(self.storage_key(), data_raw)
    }

    pub fn save(&self, storage: &mut dyn Storage, data: &T) -> StdResult<()> {
        let data_raw = C::encode(data)?;
        storage.write(self.storage_key(), &data_raw);
        Ok(())
    }

    pub fn remove(&self, storage: &mut dyn Storage) {
        storage.remove(self.storage_key());
    }

    pub fn may_update<F, E>(&self, storage: &mut dyn Storage, action: F) -> Result<T, E>
    where
        F: FnOnce(Option<T>) -> Result<T, E>,
        E: From<StdError>,
    {
        let data = action(self.may_load(storage)?)?;

        self.save(storage, &data)?;

        Ok(data)
    }

    pub fn update<F, E>(&self, storage: &mut dyn Storage, action: F) -> Result<T, E>
    where
        F: FnOnce(T) -> Result<T, E>,
        E: From<StdError>,
    {
        let data = action(self.load(storage)?)?;

        self.save(storage, &data)?;

        Ok(data)
    }

    pub fn may_modify<F, E>(&self, storage: &mut dyn Storage, action: F) -> Result<Option<T>, E>
    where
        F: FnOnce(Option<T>) -> Result<Option<T>, E>,
        E: From<StdError>,
    {
        let maybe_data = action(self.may_load(storage)?)?;

        if let Some(data) = &maybe_data {
            self.save(storage, data)?;
        } else {
            self.remove(storage);
        }

        Ok(maybe_data)
    }

    pub fn modify<F, E>(&self, storage: &mut dyn Storage, action: F) -> Result<Option<T>, E>
    where
        F: FnOnce(T) -> Result<Option<T>, E>,
        E: From<StdError>,
    {
        let maybe_data = action(self.load(storage)?)?;

        if let Some(data) = &maybe_data {
            self.save(storage, data)?;
        } else {
            self.remove(storage);
        }

        Ok(maybe_data)
    }
}

// This allows `Path` to be used in WasmRaw queries with a simplier syntax.
impl<'a, T, C> From<Path<'a, T, C>> for Binary
where
    C: Codec<T>,
{
    fn from(path: Path<'a, T, C>) -> Self {
        path.storage_key.into_owned().into()
    }
}

pub trait QuerierWrapperExt {
    fn query_wasm_raw2<T, C>(&self, contract: Addr, path: &Path<T, C>) -> StdResult<Option<T>>
    where
        C: Codec<T>;

    fn query_wasm_range<K, T, C>(
        &self,
        contract: Addr,
        prefix: Prefix<K, T, C>,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        limit: Option<u32>,
    ) -> StdResult<Box<dyn Iterator<Item = StdResult<(K::Output, T)>>>>
    where
        K: PrimaryKey,
        C: Codec<T>;
}

impl<'a> QuerierWrapperExt for QuerierWrapper<'a> {
    fn query_wasm_raw2<T, C>(&self, contract: Addr, path: &Path<T, C>) -> StdResult<Option<T>>
    where
        C: Codec<T>,
    {
        let raw = self
            .query(Query::WasmRaw(QueryWasmRawRequest {
                contract,
                key: path.storage_key().into(),
            }))?
            .as_wasm_raw();

        raw.map(|raw| C::decode(&raw)).transpose()
    }

    fn query_wasm_range<K, T, C>(
        &self,
        contract: Addr,
        prefix: Prefix<K, T, C>,
        min: Option<Bound<K>>,
        max: Option<Bound<K>>,
        limit: Option<u32>,
    ) -> StdResult<Box<dyn Iterator<Item = StdResult<(K::Output, T)>>>>
    where
        K: PrimaryKey,
        C: Codec<T>,
    {
        let (min, max) = range_bounds(&prefix.namespace, min, max);

        let res = self
            .query(Query::WasmScan(QueryWasmScanRequest {
                contract,
                min: Some(min.into()),
                max: Some(max.into()),
                limit,
            }))?
            .as_wasm_scan();

        let iter = res
            .into_iter()
            .map(move |(raw_key, raw_value)| -> StdResult<_> {
                // key contains len prefix | prefix | key.
                // We need to trim the len prefix and the prefix.
                let trimmed_key = trim(&prefix.namespace, &raw_key);
                let key = K::from_slice(&trimmed_key)?;
                let value = C::decode(&raw_value)?;
                Ok((key, value))
            });

        Ok(Box::new(iter))
    }
}

#[cfg(test)]
mod test {
    use {
        super::QuerierWrapperExt,
        crate::{Item, Map},
        grug_types::{Addr, Bound, MockQuerier, QuerierWrapper, StdResult},
        std::ops::Deref,
    };

    const MAP: Map<&str, u64> = Map::new("map");
    const ITEM: Item<u64> = Item::new("item");

    #[test]
    fn query_wasm_raw() {
        let query = MockQuerier::new().with_raw_contract_storage(Addr::mock(1), |storage| {
            MAP.save(storage, "1", &1).unwrap();
            MAP.save(storage, "2", &2).unwrap();
            MAP.save(storage, "3", &3).unwrap();
            MAP.save(storage, "4", &4).unwrap();

            ITEM.save(storage, &2).unwrap();
        });
        let querier = QuerierWrapper::new(&query);

        assert_eq!(
            querier
                .query_wasm_raw2(Addr::mock(1), &MAP.path("1"))
                .unwrap(),
            Some(1)
        );

        assert_eq!(
            querier
                .query_wasm_raw2(Addr::mock(1), ITEM.deref())
                .unwrap(),
            Some(2)
        );

        assert_eq!(
            querier
                .query_wasm_raw2(Addr::mock(1), &MAP.path("5"))
                .unwrap(),
            None
        );

        // Bound min and max
        {
            let res = querier
                .query_wasm_range(
                    Addr::mock(1),
                    MAP.no_prefix(),
                    Some(Bound::Inclusive("1")),
                    Some(Bound::Inclusive("3")),
                    None,
                )
                .unwrap()
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            assert_eq!(res, vec![
                ("1".to_string(), 1),
                ("2".to_string(), 2),
                ("3".to_string(), 3)
            ]);
        }

        // Bound min
        {
            let res = querier
                .query_wasm_range(
                    Addr::mock(1),
                    MAP.no_prefix(),
                    Some(Bound::Exclusive("2")),
                    None,
                    None,
                )
                .unwrap()
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            assert_eq!(res, vec![("3".to_string(), 3), ("4".to_string(), 4)]);
        }

        // Bound max
        {
            let res = querier
                .query_wasm_range(
                    Addr::mock(1),
                    MAP.no_prefix(),
                    None,
                    Some(Bound::Exclusive("3")),
                    None,
                )
                .unwrap()
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            assert_eq!(res, vec![("1".to_string(), 1), ("2".to_string(), 2)]);
        }
    }

    const TUPLE_MAP: Map<(&str, &str), u64> = Map::new("tuple_map");

    #[test]
    fn query_wasm_scan_tuple() {
        let query = MockQuerier::new().with_raw_contract_storage(Addr::mock(1), |storage| {
            TUPLE_MAP.save(storage, ("1", "1"), &1).unwrap();
            TUPLE_MAP.save(storage, ("1", "2"), &2).unwrap();
            TUPLE_MAP.save(storage, ("1", "3"), &3).unwrap();
            TUPLE_MAP.save(storage, ("2", "1"), &4).unwrap();
            TUPLE_MAP.save(storage, ("2", "2"), &5).unwrap();
            TUPLE_MAP.save(storage, ("2", "3"), &6).unwrap();
        });
        let querier = QuerierWrapper::new(&query);

        let res = querier
            .query_wasm_range(
                Addr::mock(1),
                TUPLE_MAP.prefix("1"),
                Some(Bound::Exclusive("1")),
                Some(Bound::Inclusive("3")),
                None,
            )
            .unwrap()
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        assert_eq!(res, vec![("2".to_string(), 2), ("3".to_string(), 3)]);
    }
}
