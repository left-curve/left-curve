use {
    crate::{Path, StdResult, Storage},
    serde::{de::DeserializeOwned, ser::Serialize},
    std::marker::PhantomData,
};

pub struct Item<'a, T> {
    storage_key: &'a [u8],
    _data_type:  PhantomData<T>,
}

impl<'a, T> Item<'a, T> {
    pub const fn new(storage_key: &'static str) -> Self {
        Self {
            storage_key: storage_key.as_bytes(),
            _data_type:  PhantomData,
        }
    }

    fn path(&self) -> Path<T> {
        Path::from_raw(self.storage_key)
    }
}

impl<'a, T> Item<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    pub fn exists(&self, store: &dyn Storage) -> bool {
        self.path().exists(store)
    }

    pub fn may_load(&self, store: &dyn Storage) -> StdResult<Option<T>> {
        self.path().may_load(store)
    }

    pub fn load(&self, store: &dyn Storage) -> anyhow::Result<T> {
        self.path().load(store)
    }

    pub fn update<A>(&self, store: &mut dyn Storage, action: A) -> anyhow::Result<Option<T>>
    where
        A: FnOnce(Option<T>) -> anyhow::Result<Option<T>>,
    {
        self.path().update(store, action)
    }

    pub fn save(&self, store: &mut dyn Storage, data: &T) -> anyhow::Result<()> {
        self.path().save(store, data)
    }

    pub fn remove(&self, store: &mut dyn Storage) {
        self.path().remove(store)
    }
}
