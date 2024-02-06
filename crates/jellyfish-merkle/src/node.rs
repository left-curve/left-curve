use cw_std::{cw_serde, hash, Hash};
use sha2::{Digest, Sha256};

const INTERNAL_NODE_HASH_PREFIX: &[u8] = &[0];
const LEAF_NODE_HASH_PERFIX:     &[u8] = &[1];

#[cw_serde]
pub enum Node {
    Internal(InternalNode),
    Leaf(LeafNode),
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        match self {
            Node::Internal(_) => false,
            Node::Leaf(_) => true,
        }
    }

    /// Computing the node's hash.
    ///
    /// To distinguish internal and leaf nodes, internal nodes are prefixed with
    /// a zero byte, leaves are prefixed with a 1 byte.
    ///
    /// If an internal nodes doesn't have a left or right child, that child is
    /// represented by a zero hash `[0u8; 32]`.
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        match self {
            Node::Internal(InternalNode { left_child, right_child }) => {
                hasher.update(INTERNAL_NODE_HASH_PREFIX);
                hasher.update(left_child.as_ref().map(|c| &c.hash).unwrap_or(&Hash::ZERO));
                hasher.update(right_child.as_ref().map(|c| &c.hash).unwrap_or(&Hash::ZERO));
            },
            Node::Leaf(LeafNode { key_hash, value_hash }) => {
                hasher.update(LEAF_NODE_HASH_PERFIX);
                hasher.update(key_hash);
                hasher.update(value_hash);
            },
        }
        Hash::from_slice(hasher.finalize().into())
    }
}

#[cw_serde]
pub struct Child {
    pub version: u64,
    pub hash:    Hash,
}

#[cw_serde]
pub struct InternalNode {
    pub left_child:  Option<Child>,
    pub right_child: Option<Child>,
}

impl InternalNode {
    /// Create an internal node without either child.
    pub fn new_childless() -> Self {
        Self {
            left_child:  None,
            right_child: None,
        }
    }
}

#[cw_serde]
pub struct LeafNode {
    key_hash:   Hash,
    value_hash: Hash,
}

impl LeafNode {
    pub fn new(key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) -> Self {
        Self {
            key_hash:   hash(key),
            value_hash: hash(value),
        }
    }
}
