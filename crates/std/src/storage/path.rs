use {
    super::nested_namespaces_with_key,
    crate::{from_json, to_json, RawKey, Storage},
    anyhow::anyhow,
    data_encoding::BASE64,
    serde::{de::DeserializeOwned, ser::Serialize},
    std::{any::type_name, marker::PhantomData},
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
    pub fn exists(&self, store: &dyn Storage) -> anyhow::Result<bool> {
        store.read(self.storage_key).map(|value| value.is_some())
    }

    pub fn may_load(&self, store: &dyn Storage) -> anyhow::Result<Option<T>> {
        store
            .read(self.storage_key)?
            .map(from_json)
            .transpose()
            .map_err(Into::into)
    }

    pub fn load(&self, store: &dyn Storage) -> anyhow::Result<T> {
        from_json(store
            .read(self.storage_key)?
            .ok_or_else(|| anyhow!(
                "[Path]: data not found! type: {}, storage key: {}",
                type_name::<T>(),
                BASE64.encode(self.storage_key),
            ))?)
            .map_err(Into::into)
    }

    pub fn update<A>(&self, store: &mut dyn Storage, action: A) -> anyhow::Result<Option<T>>
    where
        A: FnOnce(Option<T>) -> anyhow::Result<Option<T>>,
    {
        let maybe_data = action(self.may_load(store)?)?;

        if let Some(data) = &maybe_data {
            self.save(store, data)?;
        } else {
            self.remove(store)?;
        }

        Ok(maybe_data)
    }

    pub fn save(&self, store: &mut dyn Storage, data: &T) -> anyhow::Result<()> {
        let bytes = to_json(data)?;
        store.write(self.storage_key, bytes.as_ref())?;
        Ok(())
    }

    pub fn remove(&self, store: &mut dyn Storage) -> anyhow::Result<()> {
        store.remove(self.storage_key)
    }
}
