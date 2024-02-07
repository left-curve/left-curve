#[cfg(feature = "debug")]
use cw_std::Order;
use {
    crate::{Child, InternalNode, LeafNode, Node, NodeKey, Proof},
    cw_std::{hash, Batch, Hash, Item, Map, Op, Set, StdResult, Storage},
};

pub const DEFAULT_VERSION_NAMESPACE: &str = "v";
pub const DEFAULT_NODE_NAMESPACE:    &str = "n";
pub const DEFAULT_ORPHAN_NAMESPACE:  &str = "o";

/// A `Batch` the keys and values are both hashed, and collected into a slice so
/// that it can be bisected.
type HashedBatch = [(Hash, Op<Hash>)];

/// Describes the outcome of applying an op or a batch of ops at a node.
/// The node may either be updated (in which case the updated node is returned),
/// unchanged (in which case return this unchanged node), or deleted.
///
/// Note: if a Null node is unchanged (i.e. when attempting to delete a node
/// that doesn't exist) the response should be `Deleted`, not `Unchanged`.
enum OpResponse {
    Unchanged(Node),
    Updated(Node),
    Deleted,
}

/// Jellyfish Merkle tree (JMT).
///
/// Adapted from Diem's work:
/// - Whitepaper:
///   https://developers.diem.com/docs/technical-papers/jellyfish-merkle-tree-paper/
/// - Rust implementation:
///   https://github.com/diem/diem/tree/latest/storage/jellyfish-merkle
///
/// Also worth looking into:
/// - Penumbra's adaptation:
///   https://github.com/penumbra-zone/jmt
/// - Sovereign Lab's article on optimizations:
///   https://mirror.xyz/sovlabs.eth/jfx_cJ_15saejG9ZuQWjnGnG-NfahbazQH98i1J3NN8
pub struct MerkleTree<'a> {
    version: Item<'a, u64>,
    nodes:   Map<'a, &'a NodeKey, Node>,
    orphans: Set<'a, (u64, &'a NodeKey)>,
}

impl<'a> Default for MerkleTree<'a> {
    fn default() -> Self {
        Self::new(DEFAULT_VERSION_NAMESPACE, DEFAULT_NODE_NAMESPACE, DEFAULT_ORPHAN_NAMESPACE)
    }
}

#[cfg(feature = "debug")]
impl<'a> MerkleTree<'a> {
    /// Return all nodes in the tree. Useful for debugging.
    pub fn nodes(&self, store: &dyn Storage) -> StdResult<Vec<(NodeKey, Node)>> {
        self.nodes.range(store, None, None, Order::Ascending).collect()
    }

    /// Return all node keys that have been marked orphaned, and the versions
    /// since which they became orphaned. Useful for debugging.
    pub fn orphans(&self, store: &dyn Storage) -> StdResult<Vec<(u64, NodeKey)>> {
        self.orphans.range(store, None, None, Order::Ascending).collect()
    }
}

impl<'a> MerkleTree<'a> {
    /// Create a new Merkle tree with the given namespaces.
    pub const fn new(
        version_namespace: &'a str,
        node_namespace:    &'a str,
        orphan_namespace:  &'a str,
    ) -> Self {
        Self {
            version: Item::new(version_namespace),
            nodes:   Map::new(node_namespace),
            orphans: Set::new(orphan_namespace),
        }
    }

    /// Get the latest version number.
    pub fn lateset_version(&self, store: &dyn Storage) -> StdResult<u64> {
        self.version.may_load(store).map(|version| version.unwrap_or(0))
    }

    /// Get the root hash at the given version. Use latest version if unspecified.
    ///
    /// If the root node is not found at the version, return None. There are two
    /// possible reasons that it's not found: either no data has ever been
    /// written to the tree yet, or the version is old and has been pruned.
    pub fn root_hash(&self, store: &dyn Storage, version: Option<u64>) -> StdResult<Option<Hash>> {
        let version = version.map(Ok).unwrap_or_else(|| self.lateset_version(store))?;
        let root_node_key = NodeKey::root(version);
        let root_node = self.nodes.may_load(store, &root_node_key)?;
        Ok(root_node.map(|node| node.hash()))
    }

    /// Apply a batch of ops to the tree. Each op is either 1) inserting a value
    /// at a key, or 2) deleting a key.
    /// Increments the version by 1, and recomputes the root hash.
    pub fn apply(&self, store: &mut dyn Storage, batch: &Batch) -> StdResult<()> {
        let old_version = self.version.may_load(store)?.unwrap_or(0);
        let old_root_node_key = NodeKey::root(old_version);
        let new_version = old_version + 1;
        let new_root_node_key = NodeKey::root(new_version);

        // hash the keys and values
        let mut batch: Vec<_> = batch.into_iter().map(|(k, op)| (hash(k), op.as_ref().map(hash))).collect();
        // sort by key hashes
        batch.sort_by(|(k1, _), (k2, _)| k1.cmp(&k2));

        // recursively apply the ops, starting at the old root
        match self.apply_at(store, new_version, &old_root_node_key, &batch, None)? {
            OpResponse::Updated(node) | OpResponse::Unchanged(node) => {
                self.nodes.save(store, &new_root_node_key, &node)?;
            },
            OpResponse::Deleted => {}, // nothing to do
        }

        // mark the old root as orphaned, except if the old version is zero,
        if old_version > 0 {
            self.orphans.insert(store, (new_version, &old_root_node_key))?;
        }

        // save the new version. while writing the batch may not actually change
        // the root hash (e.g. if overwriting a key with the same value),
        // we still increment the version. this way, we ensure the version is
        // always the same as the block height. some logics in cw-app are based
        // on the assumption of this.
        self.version.save(store, &new_version)
    }

    fn apply_at(
        &self,
        store:         &mut dyn Storage,
        version:       u64,
        node_key:      &NodeKey,
        batch:         &HashedBatch,
        existing_leaf: Option<LeafNode>,
    ) -> StdResult<OpResponse> {
        match self.nodes.may_load(store, node_key)? {
            Some(Node::Internal(node)) => self.apply_at_internal(
                store,
                version,
                node_key,
                node,
                batch,
                existing_leaf,
            ),
            Some(Node::Leaf(node)) => {
                // we can't encounter two leaf nodes during the same insertion.
                // if our current node is a leaf, this means the existing leaf
                // must be None.
                debug_assert!(existing_leaf.is_none(), "encountered leaves twice");
                self.apply_at_leaf(store, version, node_key, node, batch)
            },
            None => self.apply_at_null(store, version, node_key, batch, existing_leaf),
        }
    }

    fn apply_at_internal(
        &self,
        store:         &mut dyn Storage,
        version:       u64,
        node_key:      &NodeKey,
        mut node:      InternalNode,
        batch:         &HashedBatch,
        existing_leaf: Option<LeafNode>,
    ) -> StdResult<OpResponse> {
        // our current depth in the tree is simply the number of bits in the
        // current node key. root node has depth 0
        let depth = node_key.bits.num_bits;

        // split the batch into two, one for left child, one for right
        let (batch_for_left, batch_for_right) = partition_batch(batch, depth);

        // if an existing leaf is given, decide whether it's going to the left
        // or the right subtree based on its bit at the current depth
        let (existing_leaf_for_left, existing_leaf_for_right) = partition_leaf(existing_leaf, depth);

        // apply at the two children, respectively
        let left_response = self.apply_at_child(
            store,
            version,
            node_key,
            true,
            node.left_child.as_ref(),
            batch_for_left,
            existing_leaf_for_left,
        )?;
        let right_response = self.apply_at_child(
            store,
            version,
            node_key,
            false,
            node.right_child.as_ref(),
            batch_for_right,
            existing_leaf_for_right,
        )?;

        match (left_response, right_response) {
            // both children are deleted. delete this node as well
            (OpResponse::Deleted, OpResponse::Deleted) => {
                Ok(OpResponse::Deleted)
            },
            // neither children is changed. this node is unchanged as well
            (OpResponse::Unchanged(_), OpResponse::Unchanged(_)) => {
                Ok(OpResponse::Unchanged(Node::Internal(node)))
            },
            // left child is deleted, right child is a leaf
            // path can be collapsed
            (OpResponse::Deleted, OpResponse::Updated(node) | OpResponse::Unchanged(node)) if node.is_leaf() => {
                Ok(OpResponse::Updated(node))
            },
            // right child is deleted, left child is a leaf
            // path can be collapsed
            (OpResponse::Updated(node) | OpResponse::Unchanged(node), OpResponse::Deleted) if node.is_leaf() => {
                Ok(OpResponse::Updated(node))
            },
            // at least one child is updated and the path can't be collapsed.
            // update the currenct node and return
            (left, right) => {
                node.left_child = match left {
                    OpResponse::Updated(child_node) => {
                        self.nodes.save(store, &node_key.child_at_version(true, version), &child_node)?;
                        Some(Child {
                            version,
                            hash: child_node.hash(),
                        })
                    },
                    OpResponse::Deleted => None,
                    OpResponse::Unchanged(_) => node.left_child,
                };

                node.right_child = match right {
                    OpResponse::Updated(child_node) => {
                        self.nodes.save(store, &node_key.child_at_version(false, version), &child_node)?;
                        Some(Child {
                            version,
                            hash: child_node.hash(),
                        })
                    },
                    OpResponse::Deleted => None,
                    OpResponse::Unchanged(_) => node.right_child,
                };

                Ok(OpResponse::Updated(Node::Internal(node)))
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_at_child(
        &self,
        store:         &mut dyn Storage,
        version:       u64,
        node_key:      &NodeKey,
        left:          bool,
        child:         Option<&Child>,
        batch:         &HashedBatch,
        existing_leaf: Option<LeafNode>,
    ) -> StdResult<OpResponse> {
        // only apply if the batch is non-empty, or an existing leaf is given
        // and it's going to this subtree
        if !batch.is_empty() || existing_leaf.is_some() {
            let child_version = child.map(|c| c.version).unwrap_or(version);
            let child_node_key = node_key.child_at_version(left, child_version);
            self.apply_at(store, version, &child_node_key, batch, existing_leaf)
        } else {
            // no action at the subtree. in this case, if there is a left child,
            // we return OpResponse::Unchanged; otherwise, return Deleted
            if let Some(child) = child {
                let child_node_key = node_key.child_at_version(left, child.version);
                Ok(OpResponse::Unchanged(self.nodes.load(store, &child_node_key)?))
            } else {
                Ok(OpResponse::Deleted)
            }
        }
    }

    fn apply_at_leaf(
        &self,
        store:    &mut dyn Storage,
        version:  u64,
        node_key: &NodeKey,
        mut node: LeafNode,
        batch:    &HashedBatch,
    ) -> StdResult<OpResponse> {
        // if there is only one op AND the key matches exactly the leaf node's
        // key, then we have found the correct node, apply the op right here.
        if batch.len() == 1 {
            let (bits, op) = &batch[0];
            if *bits == node.key_hash {
                return if let Op::Put(value) = op {
                    let new_value_hash = hash(value);
                    if node.value_hash == new_value_hash {
                        Ok(OpResponse::Unchanged(Node::Leaf(node)))
                    } else {
                        node.value_hash = new_value_hash;
                        Ok(OpResponse::Updated(Node::Leaf(node)))
                    }
                } else {
                    Ok(OpResponse::Deleted)
                };
            }
        }

        // now we know we can't apply the op right at this leaf node, either
        // because there are more than one ops in the batch, or there is only
        // one but it doesn't match the current leaf. either way, we have to
        // turn the current node into an internal node and apply the batch at
        // its children.
        let new_internal_node = InternalNode::new_childless();
        self.apply_at_internal(store, version, node_key, new_internal_node, batch, Some(node))
    }

    fn apply_at_null(
        &self,
        store:         &mut dyn Storage,
        version:       u64,
        node_key:      &NodeKey,
        batch:         &HashedBatch,
        existing_leaf: Option<LeafNode>,
    ) -> StdResult<OpResponse> {
        match batch.len() {
            // batch is empty, the existing leaf must be given (otherwise this
            // function shouldn't have been called). in this case we create a
            // new leaf node
            0 => {
                debug_assert!(
                    existing_leaf.is_some(),
                    "apply_at_null called when batch and existing_leaf are both empty"
                );
                let new_leaf_node = existing_leaf.unwrap();
                Ok(OpResponse::Updated(Node::Leaf(new_leaf_node)))
            },
            // there is exactly one op to do
            // if it's an insert, create a new leaf node
            // if it's an delete, nothing to do (deleting a non-exist node)
            1 => Ok({
                let (key_hash, op) = &batch[0];
                match op {
                    Op::Put(value_hash) => {
                        let new_leaf_node = LeafNode::new(key_hash.clone(), value_hash.clone());
                        OpResponse::Updated(Node::Leaf(new_leaf_node))
                    },
                    Op::Delete => OpResponse::Deleted,
                }
            }),
            // there are more than one op to do. create an empty internal node
            // and apply the batch at this internal node
            _ => {
                let node = InternalNode::new_childless();
                self.apply_at_internal(store, version, node_key, node, batch, existing_leaf)
            },
        }
    }

    /// Generate Merkle proof for the a key at the given version. If the key
    /// exists in the tree, a membership proof is returned; otherwise,
    /// a non-membership proof is returned. If version is isn't specified, use
    /// the latest version.
    pub fn prove(
        &self,
        _store:    &dyn Storage,
        _key_hash: &Hash,
        _version:  Option<u64>,
    ) -> StdResult<Proof> {
        todo!()
    }

    /// Delete nodes that no longer part of the tree since `up_to_version`
    /// (exclusive). If no `up_to_version` is provided then delete all orphans.
    pub fn prune(
        &self,
        _store:         &mut dyn Storage,
        _up_to_version: Option<u64>,
        _limit:         Option<usize>,
    ) -> StdResult<()> {
        todo!()
    }
}

/// TODO: add explanation on what this does
fn partition_batch(batch: &HashedBatch, depth: usize) -> (&HashedBatch, &HashedBatch) {
    let partition_point = batch.partition_point(|(key_hash, _)| {
        bit_at_index(key_hash, depth) == 0
    });
    (&batch[..partition_point], &batch[partition_point..])
}

/// TODO: add explanation on what this does
fn partition_leaf(leaf: Option<LeafNode>, depth: usize) -> (Option<LeafNode>, Option<LeafNode>) {
    if let Some(leaf) = leaf {
        // TODO: avoid cloning here
        let bit = bit_at_index(&leaf.key_hash, depth);
        // 0 = left, 1 = right
        debug_assert!(bit == 0 || bit == 1);
        if bit == 0 {
            (Some(leaf), None)
        } else {
            (None, Some(leaf))
        }
    } else {
        (None, None)
    }
}

/// Get the i-th bit without having to cast the byte slice to BitArray (which
/// involves some copying).
fn bit_at_index(bytes: &[u8], index: usize) -> u8 {
    let (quotient, remainder) = (index / 8, index % 8);
    let byte = bytes[quotient];
    (byte >> (7 - remainder)) & 0b1
}
