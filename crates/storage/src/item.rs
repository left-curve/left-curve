use {
    crate::{Borsh, Codec, Path},
    grug_types::{StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct Item<'a, T, C = Borsh>
where
    C: Codec<T>,
{
    storage_key: &'a [u8],
    data: PhantomData<T>,
    codec: PhantomData<C>,
}

impl<'a, T, C> Item<'a, T, C>
where
    C: Codec<T>,
{
    pub const fn new(storage_key: &'a str) -> Self {
        Self {
            storage_key: storage_key.as_bytes(),
            data: PhantomData,
            codec: PhantomData,
        }
    }

    fn path(&self) -> Path<T, C> {
        Path::from_raw(self.storage_key)
    }

    pub fn exists(&self, storage: &dyn Storage) -> bool {
        self.path().exists(storage)
    }

    pub fn may_load_raw(&self, storage: &dyn Storage) -> Option<Vec<u8>> {
        self.path().may_load_raw(storage)
    }

    pub fn may_load(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        self.path().may_load(storage)
    }

    pub fn load_raw(&self, storage: &dyn Storage) -> StdResult<Vec<u8>> {
        self.path().load_raw(storage)
    }

    pub fn load(&self, storage: &dyn Storage) -> StdResult<T> {
        self.path().load(storage)
    }

    pub fn unsafe_save_raw(&self, storage: &mut dyn Storage, data_raw: &[u8]) {
        self.path().save_raw(storage, data_raw)
    }

    pub fn save(&self, storage: &mut dyn Storage, data: &T) -> StdResult<()> {
        self.path().save(storage, data)
    }

    pub fn remove(&self, storage: &mut dyn Storage) {
        self.path().remove(storage)
    }

    pub fn update<A, Err>(&self, storage: &mut dyn Storage, action: A) -> Result<Option<T>, Err>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, Err>,
        Err: From<StdError>,
    {
        self.path().update(storage, action)
    }
}
