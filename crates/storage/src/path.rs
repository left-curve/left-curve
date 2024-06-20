use {
    crate::{Borsh, Encoding},
    grug_types::{nested_namespaces_with_key, StdError, StdResult, Storage},
    std::{borrow::Cow, marker::PhantomData},
};

pub struct PathBuf<T, E: Encoding<T> = Borsh> {
    storage_key: Vec<u8>,
    data: PhantomData<T>,
    encoding: PhantomData<E>,
}

impl<T, E> PathBuf<T, E>
where
    E: Encoding<T>,
{
    pub fn new(namespace: &[u8], prefixes: &[Cow<[u8]>], maybe_key: Option<&Cow<[u8]>>) -> Self {
        Self {
            storage_key: nested_namespaces_with_key(Some(namespace), prefixes, maybe_key),
            data: PhantomData,
            encoding: PhantomData,
        }
    }

    pub fn as_path(&self) -> Path<'_, T, E> {
        Path {
            storage_key: self.storage_key.as_slice(),
            data: self.data,
            encoding: self.encoding,
        }
    }
}

pub struct Path<'a, T, E: Encoding<T> = Borsh> {
    storage_key: &'a [u8],
    data: PhantomData<T>,
    encoding: PhantomData<E>,
}

impl<'a, T, E> Path<'a, T, E>
where
    E: Encoding<T>,
{
    pub(crate) fn from_raw(storage_key: &'a [u8]) -> Self {
        Self {
            storage_key,
            data: PhantomData,
            encoding: PhantomData,
        }
    }

    pub fn exists(&self, storage: &dyn Storage) -> bool {
        storage.read(self.storage_key).is_some()
    }

    pub fn may_load_raw(&self, storage: &dyn Storage) -> Option<Vec<u8>> {
        storage.read(self.storage_key)
    }

    pub fn may_load(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        storage
            .read(self.storage_key)
            .map(|val| E::decode(&val))
            .transpose()
    }

    pub fn load_raw(&self, storage: &dyn Storage) -> StdResult<Vec<u8>> {
        storage
            .read(self.storage_key)
            .ok_or_else(|| StdError::data_not_found::<T>(self.storage_key))
    }

    pub fn load(&self, storage: &dyn Storage) -> StdResult<T> {
        storage
            .read(self.storage_key)
            .ok_or_else(|| StdError::data_not_found::<T>(self.storage_key))
            .and_then(|val| E::decode(&val))
    }

    pub fn save_raw(&self, storage: &mut dyn Storage, data_raw: &[u8]) {
        storage.write(self.storage_key, data_raw)
    }

    pub fn save(&self, storage: &mut dyn Storage, data: &T) -> StdResult<()> {
        let data_raw = E::encode(data)?;
        storage.write(self.storage_key, &data_raw);
        Ok(())
    }

    pub fn remove(&self, storage: &mut dyn Storage) {
        storage.remove(self.storage_key);
    }

    pub fn update<A, Err>(&self, storage: &mut dyn Storage, action: A) -> Result<Option<T>, Err>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, Err>,
        Err: From<StdError>,
    {
        let maybe_data = action(self.may_load(storage)?)?;

        if let Some(data) = &maybe_data {
            self.save(storage, data)?;
        } else {
            self.remove(storage);
        }

        Ok(maybe_data)
    }
}
