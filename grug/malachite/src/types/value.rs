use grug_types::Hash256;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Value(Hash256);

impl malachitebft_core_types::Value for Value {
    type Id = Hash256;

    fn id(&self) -> Self::Id {
        self.0
    }
}
