use {
    crate::{from_json, nested_namespaces_with_key, to_json, RawKey, StdError, StdResult, Storage},
    serde::{de::DeserializeOwned, ser::Serialize},
    std::marker::PhantomData,
};

pub struct PathBuf<T> {
    storage_key: Vec<u8>,
    _data_type:  PhantomData<T>,
}

impl<T> PathBuf<T> {
    pub fn new(namespace: &[u8], prefixes: &[RawKey], maybe_key: Option<&RawKey>) -> Self {
        Self {
            storage_key: nested_namespaces_with_key(Some(namespace), prefixes, maybe_key),
            _data_type:  PhantomData,
        }
    }

    pub fn as_path(&self) -> Path<'_, T> {
        Path {
            storage_key: self.storage_key.as_slice(),
            _data_type:  self._data_type,
        }
    }
}

pub struct Path<'a, T> {
    storage_key: &'a [u8],
    _data_type:  PhantomData<T>,
}

impl<'a, T> Path<'a, T> {
    pub(crate) fn from_raw(storage_key: &'a [u8]) -> Self {
        Self {
            storage_key,
            _data_type: PhantomData,
        }
    }
}

impl<'a, T> Path<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn exists(&self, store: &dyn Storage) -> bool {
        store.read(self.storage_key).is_some()
    }

    pub fn may_load(&self, store: &dyn Storage) -> StdResult<Option<T>> {
        store.read(self.storage_key).map(from_json).transpose()
    }

    pub fn load(&self, store: &dyn Storage) -> StdResult<T> {
        from_json(store
            .read(self.storage_key)
            .ok_or_else(|| StdError::data_not_found::<T>(self.storage_key))?)
            .map_err(Into::into)
    }

    // compared to the original cosmwasm, we require `action` to return an
    // option, which in case of None leads to the record being deleted.
    pub fn update<A, E>(&self, store: &mut dyn Storage, action: A) -> Result<Option<T>, E>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, E>,
        E: From<StdError>,
    {
        let maybe_data = action(self.may_load(store)?)?;

        if let Some(data) = &maybe_data {
            self.save(store, data)?;
        } else {
            self.remove(store);
        }

        Ok(maybe_data)
    }

    pub fn save(&self, store: &mut dyn Storage, data: &T) -> StdResult<()> {
        let bytes = to_json(data)?;
        store.write(self.storage_key, &bytes);
        Ok(())
    }

    pub fn remove(&self, store: &mut dyn Storage) {
        store.remove(self.storage_key);
    }
}
