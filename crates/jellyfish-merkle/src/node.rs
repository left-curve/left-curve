use cw_std::{cw_serde, Hash};

#[cw_serde]
pub enum Node {
    Internal {
        left_hash:  Hash,
        right_hash: Hash,
    },
    Leaf {
        key_hash:   Hash,
        value_hash: Hash,
    },
}
