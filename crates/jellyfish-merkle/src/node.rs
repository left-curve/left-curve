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
            Node::Internal(internal_node) => internal_node.hash(),
            Node::Leaf(leaf_node) => leaf_node.hash(),
        }
    }
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
        let mut hasher = Sha256::new();
        hasher.update(INTERNAL_NODE_HASH_PREFIX);
        hasher.update(self.left_child.as_ref().map(|c| &c.hash).unwrap_or(&Hash::ZERO));
        hasher.update(self.right_child.as_ref().map(|c| &c.hash).unwrap_or(&Hash::ZERO));
        Hash::from_slice(hasher.finalize().into())
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
        let mut hasher = Sha256::new();
        hasher.update(LEAF_NODE_HASH_PERFIX);
        hasher.update(&self.key_hash);
        hasher.update(&self.value_hash);
        Hash::from_slice(hasher.finalize().into())
    }
}
