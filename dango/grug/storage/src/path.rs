use {
    crate::{Codec, RawKey},
    grug_types::{Binary, StdError, StdResult, Storage, nested_namespaces_with_key},
    std::{borrow::Cow, marker::PhantomData},
};

pub struct Path<'a, T, C> {
    storage_key: Cow<'a, [u8]>,
    data: PhantomData<T>,
    codec: PhantomData<C>,
}

impl<T, C> Clone for Path<'_, T, C> {
    fn clone(&self) -> Self {
        Self {
            storage_key: self.storage_key.clone(),
            data: PhantomData,
            codec: PhantomData,
        }
    }
}

impl<'a, T, C> Path<'a, T, C>
where
    C: Codec<T>,
{
    pub fn new(namespace: &[u8], prefixes: &[RawKey], maybe_key: Option<RawKey>) -> Self {
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
        let maybe_current = self.may_load(storage)?;
        let was_present = maybe_current.is_some();

        let maybe_data = action(maybe_current)?;

        match (&maybe_data, was_present) {
            (Some(data), _) => {
                self.save(storage, data)?;
            },
            (None, true) => {
                self.remove(storage);
            },
            (None, false) => {},
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
