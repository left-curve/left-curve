use {
    cw_std::{cw_serde, Hash},
    sha2::{Digest, Sha256},
};

const INTERNAL_NODE_HASH_PREFIX: &[u8] = &[0];
const LEAF_NODE_HASH_PERFIX:     &[u8] = &[1];

// ----------------------------------- node ------------------------------------

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
        match self {
            Node::Internal(InternalNode { left_child, right_child }) => {
                hash_internal_node(hash_of(left_child), hash_of(right_child))
            },
            Node::Leaf(LeafNode { key_hash, value_hash }) => {
                hash_leaf_node(key_hash, value_hash)
            },
        }
    }
}

pub fn hash_internal_node(left_hash: Option<&Hash>, right_hash: Option<&Hash>) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(INTERNAL_NODE_HASH_PREFIX);
    hasher.update(left_hash.unwrap_or(&Hash::ZERO));
    hasher.update(right_hash.unwrap_or(&Hash::ZERO));
    Hash::from_slice(hasher.finalize().into())
}

pub fn hash_leaf_node(key_hash: &Hash, value_hash: &Hash) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(LEAF_NODE_HASH_PERFIX);
    hasher.update(key_hash);
    hasher.update(value_hash);
    Hash::from_slice(hasher.finalize().into())
}

// just a helper function to avoid repetitive verbose code...
#[inline]
fn hash_of(child: &Option<Child>) -> Option<&Hash> {
    child.as_ref().map(|child| &child.hash)
}

// ------------------------------- internal node -------------------------------

#[cw_serde]
pub struct InternalNode {
    pub left_child:  Option<Child>,
    pub right_child: Option<Child>,
}

#[cw_serde]
pub struct Child {
    pub version: u64,
    pub hash:    Hash,
}

impl InternalNode {
    /// Create an internal node without either child.
    pub fn new_childless() -> Self {
        Self {
            left_child:  None,
            right_child: None,
        }
    }

    pub fn hash(&self) -> Hash {
        hash_internal_node(hash_of(&self.left_child), hash_of(&self.right_child))
    }
}

// --------------------------------- leaf node ---------------------------------

#[cw_serde]
pub struct LeafNode {
    pub key_hash:   Hash,
    pub value_hash: Hash,
}

impl LeafNode {
    pub fn new(key_hash: Hash, value_hash: Hash) -> Self {
        Self {
            key_hash,
            value_hash,
        }
    }

    pub fn hash(&self) -> Hash {
        hash_leaf_node(&self.key_hash, &self.value_hash)
    }
}
