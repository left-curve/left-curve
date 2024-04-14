use {
    crate::Path,
    borsh::{BorshDeserialize, BorshSerialize},
    cw_types::{StdError, StdResult, Storage},
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
    pub fn exists(&self, store: &dyn Storage) -> bool {
        self.path().exists(store)
    }

    pub fn may_load(&self, store: &dyn Storage) -> StdResult<Option<T>> {
        self.path().may_load(store)
    }

    pub fn load(&self, store: &dyn Storage) -> StdResult<T> {
        self.path().load(store)
    }

    pub fn update<A, E>(&self, store: &mut dyn Storage, action: A) -> Result<Option<T>, E>
    where
        A: FnOnce(Option<T>) -> Result<Option<T>, E>,
        E: From<StdError>,
    {
        self.path().update(store, action)
    }

    pub fn save(&self, store: &mut dyn Storage, data: &T) -> StdResult<()> {
        self.path().save(store, data)
    }

    pub fn remove(&self, store: &mut dyn Storage) {
        self.path().remove(store)
    }
}
