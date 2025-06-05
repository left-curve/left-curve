use grug::Hash256;

#[grug::derive(Borsh)]
#[derive(Ord, PartialOrd)]
pub struct Value(Hash256);

impl malachitebft_core_types::Value for Value {
    type Id = Hash256;

    fn id(&self) -> Self::Id {
        self.0
    }
}
