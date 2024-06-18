use {
    crate::{Borsh, Encoding, Path},
    grug_types::{StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct Item<'a, T, E: Encoding<T> = Borsh> {
    storage_key: &'a [u8],
    data: PhantomData<T>,
    encoding: PhantomData<E>,
}

impl<'a, T, E> Item<'a, T, E>
where
    E: Encoding<T>,
{
    pub const fn new(storage_key: &'a str) -> Self {
        Self {
            storage_key: storage_key.as_bytes(),
            data: PhantomData,
            encoding: PhantomData,
        }
    }

    fn path(&self) -> Path<T, E> {
        Path::from_raw(self.storage_key)
    }

    pub fn exists(&self, storage: &dyn Storage) -> bool {
        self.path().exists(storage)
    }

    pub fn remove(&self, storage: &mut dyn Storage) {
        self.path().remove(storage)
    }
}

impl<'a, T, E> Item<'a, T, E>
where
    E: Encoding<T>,
{
    pub fn save(&self, storage: &mut dyn Storage, data: &T) -> StdResult<()> {
        self.path().save(storage, data)
    }

    pub fn may_load(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        self.path().may_load(storage)
    }

    pub fn load(&self, storage: &dyn Storage) -> StdResult<T> {
        self.path().load(storage)
    }

    pub fn update<A, Err>(&self, storage: &mut dyn Storage, action: A) -> Result<Option<T>, Err>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, Err>,
        Err: From<StdError>,
    {
        self.path().update(storage, action)
    }
}
