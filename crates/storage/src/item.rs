use {
    crate::{Borsh, Encoding, Path},
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct Item<'a, T, E: Encoding = Borsh> {
    storage_key: &'a [u8],
    data: PhantomData<T>,
    encoding: PhantomData<E>,
}

impl<'a, T, E> Item<'a, T, E>
where
    E: Encoding,
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

impl<'a, T> Item<'a, T, Borsh>
where
    T: BorshSerialize,
{
    pub fn save(&self, storage: &mut dyn Storage, data: &T) -> StdResult<()> {
        self.path().save(storage, data)
    }
}

impl<'a, T> Item<'a, T, Borsh>
where
    T: BorshDeserialize,
{
    pub fn may_load(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        self.path().may_load(storage)
    }

    pub fn load(&self, storage: &dyn Storage) -> StdResult<T> {
        self.path().load(storage)
    }
}

impl<'a, T> Item<'a, T>
where
    T: BorshSerialize + BorshDeserialize,
{
    pub fn update<A, E>(&self, storage: &mut dyn Storage, action: A) -> Result<Option<T>, E>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, E>,
        E: From<StdError>,
    {
        self.path().update(storage, action)
    }
}
