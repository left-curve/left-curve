use grug::Hash256;

#[grug::derive(Borsh)]
#[derive(Ord, PartialOrd)]
pub struct Value(Hash256);

impl Value {
    pub fn new(hash: Hash256) -> Self {
        Self(hash)
    }
}

impl<T> From<T> for Value
where
    T: Into<[u8; 32]>,
{
    fn from(hash: T) -> Self {
        Self(Hash256::from_inner(hash.into()))
    }
}

impl malachitebft_core_types::Value for Value {
    type Id = Hash256;

    fn id(&self) -> Self::Id {
        self.0
    }
}
