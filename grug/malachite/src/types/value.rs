use crate::types::BlockHash;

#[grug::derive(Borsh)]
#[derive(Ord, PartialOrd)]
pub struct Value(BlockHash);

impl Value {
    pub fn new(hash: BlockHash) -> Self {
        Self(hash)
    }
}

impl malachitebft_core_types::Value for Value {
    type Id = BlockHash;

    fn id(&self) -> Self::Id {
        self.0
    }
}
