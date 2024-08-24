use {
    crate::{Borsh, Codec, Path},
    std::ops::Deref,
};

pub struct Item<'a, T, C = Borsh>
where
    C: Codec<T>,
{
    path: Path<'a, T, C>,
}

impl<'a, T, C> Item<'a, T, C>
where
    C: Codec<T>,
{
    pub const fn new(storage_key: &'a str) -> Self {
        Self {
            path: Path::from_raw(storage_key.as_bytes()),
        }
    }
}

impl<'a, T, C: Codec<T>> Deref for Item<'a, T, C> {
    type Target = Path<'a, T, C>;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}
