use {
    crate::Path,
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{StdError, StdResult, Storage},
    std::marker::PhantomData,
};

pub struct Item<'a, T> {
    storage_key: &'a [u8],
    _data_type:  PhantomData<T>,
}

impl<'a, T> Item<'a, T> {
    pub const fn new(storage_key: &'a str) -> Self {
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
    T: BorshSerialize + BorshDeserialize,
{
    pub fn exists(&self, storage: &dyn Storage) -> bool {
        self.path().exists(storage)
    }

    pub fn may_load(&self, storage: &dyn Storage) -> StdResult<Option<T>> {
        self.path().may_load(storage)
    }

    pub fn load(&self, storage: &dyn Storage) -> StdResult<T> {
        self.path().load(storage)
    }

    pub fn update<A, E>(&self, storage: &mut dyn Storage, action: A) -> Result<Option<T>, E>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, E>,
        E: From<StdError>,
    {
        self.path().update(storage, action)
    }

    pub fn save(&self, storage: &mut dyn Storage, data: &T) -> StdResult<()> {
        self.path().save(storage, data)
    }

    pub fn remove(&self, storage: &mut dyn Storage) {
        self.path().remove(storage)
    }
}
