use cw_std::{cw_serde, Hash};
use sha2::{Digest, Sha256};

const INTERNAL_NODE_HASH_PREFIX: &[u8] = &[0];
const LEAF_NODE_HASH_PERFIX:     &[u8] = &[1];

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

impl Node {
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        match self {
            Node::Internal { left_hash, right_hash } => {
                hasher.update(INTERNAL_NODE_HASH_PREFIX);
                hasher.update(left_hash);
                hasher.update(right_hash);
            },
            Node::Leaf { key_hash, value_hash } => {
                hasher.update(LEAF_NODE_HASH_PERFIX);
                hasher.update(key_hash);
                hasher.update(value_hash);
            },
        }
        Hash::from_slice(hasher.finalize().into())
    }
}
