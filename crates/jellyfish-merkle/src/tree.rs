use {
    crate::{BitArray, Child, InternalNode, LeafNode, Node, NodeKey, Proof, ProofNode},
    cw_std::{hash, Batch, Hash, Item, Map, Op, Order, Set, StdResult, Storage},
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

    /// Apply a batch of ops to the tree, recomputes the root hash, and increment
    /// the version if root hash is changed.
    ///
    /// Each of is either 1) inserting a value at a key, or 2) deleting a key.
    ///
    /// This function takes a batch where both the keys and values are prehashes.
    /// If you already have them hashed and sorted ascendingly by the key hashes,
    /// use `apply` instead.
    pub fn apply_raw(&self, store: &mut dyn Storage, batch: &Batch) -> StdResult<()> {
        // hash the keys and values
        let mut batch: Vec<_> = batch.iter().map(|(k, op)| (hash(k), op.as_ref().map(hash))).collect();

        // sort by key hashes ascendingly
        batch.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

        self.apply(store, &batch)
    }

    /// Apply a batch of ops to the tree, recomputes the root hash, and increment
    /// the version if root hash is changed.
    ///
    /// Each op is either 1) inserting a value at a key, or 2) deleting a key.
    ///
    /// This function takes a `HashedBatch` where both the keys and values are
    /// hashed, and sorted ascendingly by the key hashes. If you have a batch
    /// of prehashes, use `apply_raw` instead.
    pub fn apply(&self, store: &mut dyn Storage, batch: &HashedBatch) -> StdResult<()> {
        let old_version = self.lateset_version(store)?;
        let old_root_node_key = NodeKey::root(old_version);
        let new_version = old_version + 1;
        let new_root_node_key = NodeKey::root(new_version);

        // recursively apply the ops, starting at the old root
        match self.apply_at(store, new_version, &old_root_node_key, batch, None)? {
            OpResponse::Updated(node) => {
                // increment the version
                self.version.save(store, &new_version)?;

                // save the updated root node
                self.nodes.save(store, &new_root_node_key, &node)?;

                // mark the old root as orphaned, except if the old version is
                // zero, because at version zero there isn't a root node
                if old_version > 0 {
                    self.orphans.insert(store, (new_version, &old_root_node_key))?;
                }
            },
            OpResponse::Deleted => {
                self.version.save(store, &new_version)?;
                if old_version > 0 {
                    self.orphans.insert(store, (new_version, &old_root_node_key))?;
                }
            },
            OpResponse::Unchanged(_) => {
                // nothing to do. we only increment the version if the tree has
                // been changed at all.
            },
        }

        Ok(())
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
            Some(Node::Leaf(node)) => {
                // we can't run into leaf nodes twice during the same insertion.
                // if our current node is a leaf, this means the existing leaf
                // must be None.
                debug_assert!(existing_leaf.is_none(), "encountered leaves twice");
                self.apply_at_leaf(store, version, node_key, node, batch)
            },
            Some(Node::Internal(node)) => {
                self.apply_at_internal( store, version, node_key, node, batch, existing_leaf)
            },
            None => {
                self.apply_at_null(store, version, node_key, batch, existing_leaf)
            },
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
        // if the batch is non-empty OR there's an existing leaf to be inserted,
        // then we recursively apply at the child node
        if !batch.is_empty() || existing_leaf.is_some() {
            let child_version = child.map(|c| c.version).unwrap_or(version);
            let child_node_key = node_key.child_at_version(left, child_version);
            return self.apply_at(store, version, &child_node_key, batch, existing_leaf);
        }

        // the batch is empty AND there isn't an existing leaf to be inserted.
        // in other words, there's nothing to be done. we end the recursion here.
        // if the child exists, we return Unchanged; otherwise, return Deleted
        if let Some(child) = child {
            let child_node_key = node_key.child_at_version(left, child.version);
            let child_node = self.nodes.load(store, &child_node_key)?;
            return Ok(OpResponse::Unchanged(child_node));
        }

        Ok(OpResponse::Deleted)
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
                return if let Op::Put(value_hash) = op {
                    if node.value_hash == *value_hash {
                        Ok(OpResponse::Unchanged(Node::Leaf(node)))
                    } else {
                        node.value_hash = value_hash.clone();
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
        match (batch.len(), existing_leaf) {
            // batch is empty, and there isn't an existing leaf.
            // this situation should not happen. in this case, the function
            // would have never been called (see the logics in `apply_at_child`)
            (0, None) => {
                unreachable!("applying an empty batch with no existing leaf");
            },
            // batch is empty, but there's an existing leaf.
            // in this case, we create a new leaf node
            (0, Some(leaf_node)) => {
                Ok(OpResponse::Updated(Node::Leaf(leaf_node)))
            },
            // there is exactly one op to do, and no existing leaf.
            // if it's an insert, create a new leaf node;
            // if it's an delete, nothing to do (deleting a non-exist node).
            (1, None) => Ok({
                let (key_hash, op) = &batch[0];
                match op {
                    Op::Put(value_hash) => {
                        let new_leaf_node = LeafNode::new(key_hash.clone(), value_hash.clone());
                        OpResponse::Updated(Node::Leaf(new_leaf_node))
                    },
                    Op::Delete => OpResponse::Deleted,
                }
            }),
            // there are more than one op to do.
            // regardless of whether there's an existing leaf, create an empty
            // internal node and apply the batch at this internal node.
            (_, existing_leaf) => {
                let new_internal_node = InternalNode::new_childless();
                self.apply_at_internal(store, version, node_key, new_internal_node, batch, existing_leaf)
            },
        }
    }

    /// Generate Merkle proof for the a key at the given version.
    ///
    /// If the key exists in the tree, a membership proof is returned;
    /// otherwise, a non-membership proof is returned.
    ///
    /// If version is isn't specified, use the latest version.
    ///
    /// Note that this method only looks at the key, not the value. Therefore
    /// it may be possible that the caller thinks key A exists with value B,
    /// while in fact it exists with value C, in which case this method will
    /// succeed, returning a proof for the membership of (A, C). If the caller
    /// then attempts to verify the proof with (A, B) it will fail.
    ///
    /// The intended way to avoid this situation is to use a raw key-value store
    /// together with the Merkle tree:
    ///
    /// - raw KV store stores prehash keys + prehash values
    /// - Merkle tree stores hashed keys + hashed values
    ///
    /// To query a key with proof, the caller should first call `get` on the raw
    /// KV store, then `prove` on the Merkle tree. This separation of data
    /// storage and data commitment was put forward by Cosmos SDK's ADR-65:
    /// https://github.com/cosmos/cosmos-sdk/blob/main/docs/architecture/adr-065-store-v2.md
    pub fn prove(
        &self,
        store:    &dyn Storage,
        key_hash: &Hash,
        version:  Option<u64>,
    ) -> StdResult<Proof> {
        let version = version.map(Ok).unwrap_or_else(|| self.lateset_version(store))?;
        let mut node_key = NodeKey::root(version);
        // TODO: add a more descriptive error message if the root is not found
        // e.g. "root node not found at version x (latest version: y), probably pruned"
        let mut node = self.nodes.load(store, &node_key)?;

        let bitarray = BitArray::from_bytes(key_hash);
        let mut bits = bitarray.range(None, None, Order::Ascending);
        let mut proof_node = None;
        let mut sibling_hashes = vec![];

        loop {
            match node {
                // we've reached a leaf node. if the key hashes match then we've
                // found it. if they don't match then we know the key doesn't
                // doesn't exist in the tree. either way, break the loop.
                Node::Leaf(leaf) => {
                    if *key_hash != leaf.key_hash {
                        proof_node = Some(ProofNode::Leaf {
                            key_hash:   leaf.key_hash,
                            value_hash: leaf.value_hash,
                        });
                    }
                    break;
                },
                // we've reached an internal node. move on to its child based on
                // the next bit in the key hash. append its sibling to the sibling
                // hashes.
                Node::Internal(InternalNode { left_child, right_child }) => {
                    match (bits.next(), left_child, right_child) {
                        (Some(0), Some(child), sibling) => {
                            sibling_hashes.push(hash_of(sibling));
                            node_key = node_key.child_at_version(true, child.version);
                            node = self.nodes.load(store, &node_key)?;
                        },
                        (Some(1), sibling, Some(child)) => {
                            sibling_hashes.push(hash_of(sibling));
                            node_key = node_key.child_at_version(false, child.version);
                            node = self.nodes.load(store, &node_key)?;
                        },
                        (Some(0), None, sibling) => {
                            proof_node = Some(ProofNode::Internal {
                                left_hash:  None,
                                right_hash: hash_of(sibling),
                            });
                            break;
                        },
                        (Some(1), sibling, None) => {
                            proof_node = Some(ProofNode::Internal {
                                left_hash:  hash_of(sibling),
                                right_hash: None,
                            });
                            break;
                        },
                        (bit, _, _) => {
                            // the next bit must exist, because if we have reached the end of the
                            // bitarray, the node is definitely a leaf. also it can only be 0 or 1.
                            unreachable!("unexpected next bit: {bit:?}");
                        },
                    };
                },
            }
        }

        // in our proof format, the sibling hashes are from bottom up (from leaf
        // to the root), so we have to reverse it.
        // we can either reverse it during proving, or during verification.
        // we do it here since proving is usually done off-chain (e.g. an IBC
        // relayer querying the node) while verification is usally done on-chain
        // (e.g. inside an IBC light client).
        sibling_hashes.reverse();

        if let Some(node) = proof_node {
            Ok(Proof::NonMembership { node, sibling_hashes })
        } else {
            Ok(Proof::Membership { sibling_hashes })
        }
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

fn partition_batch(batch: &HashedBatch, depth: usize) -> (&HashedBatch, &HashedBatch) {
    let partition_point = batch.partition_point(|(key_hash, _)| {
        bit_at_index(key_hash, depth) == 0
    });
    (&batch[..partition_point], &batch[partition_point..])
}

fn partition_leaf(leaf: Option<LeafNode>, depth: usize) -> (Option<LeafNode>, Option<LeafNode>) {
    if let Some(leaf) = leaf {
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

// just a helper function to avoid repetitive verbose code...
#[inline]
fn hash_of(child: Option<Child>) -> Option<Hash> {
    child.map(|child| child.hash)
}

// ----------------------------------- tests -----------------------------------

// we use the following very simple merkle tree in these tests:
// (parentheses designates internal nodes. without parentheses then it's a leaf)
//
//           root
//         ┌──┴──┐
//        (0)    1
//      ┌──┴──┐
//    null   (01)
//         ┌──┴──┐
//       (010)  (011)
//            ┌──┴──┐
//          0110   0111
//
// to build this tree, we need four keys that hash to 010..., 0110..., 0111...,
// and 1... respectively, which were found with a little trials:
//
// sha256("r") = 0100...
// sha256("m") = 0110...
// sha256("L") = 0111...
// sha256("a") = 1100...
//
// the node hashes are computed as follows:
//
// hash of node 0110
// = sha256(01 | sha256("m") | sha256("bar"))
// = sha256(01 | 62c66a7a5dd70c3146618063c344e531e6d4b59e379808443ce962b3abd63c5a | fcde2b2edba56bf408601fb721fe9b5c338d10ee429ea04fae5511b68fbf8fb9)
// = fd34e3f8d9840e7f6d6f639435b6f9b67732fc5e3d5288e268021aeab873f280
//
// hash of node 0111
// = sha256(01 | sha256("L") | sha256("fuzz"))
// = sha256(01 | 72dfcfb0c470ac255cde83fb8fe38de8a128188e03ea5ba5b2a93adbea1062fa | 93850b707585e404e4951a3ddc1f05a34b3d4f5fc081d616f46d8a2e8f1c8e68)
// = 412341380b1e171077dd9da9af936ae2126ede2dd91dc5acb0f77363d46eb76b
//
// hash of node 011
// = sha256(00 | fd34e3f8d9840e7f6d6f639435b6f9b67732fc5e3d5288e268021aeab873f280 | 412341380b1e171077dd9da9af936ae2126ede2dd91dc5acb0f77363d46eb76b)
// = e104e2bcf24027af737c021033cb9d8cbd710a463f54ae6f2ff9eb06c784c744
//
// hash of node 010
// = sha256(01 | sha256("r") | sha256("foo"))
// = sha256(01 | 454349e422f05297191ead13e21d3db520e5abef52055e4964b82fb213f593a1 | 2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae)
// = c8348e9a7a327e8b76e97096c362a1f87071ee4108b565d1f409529c189cb684
//
// hash of node 01
// = sha256(00 | c8348e9a7a327e8b76e97096c362a1f87071ee4108b565d1f409529c189cb684 | e104e2bcf24027af737c021033cb9d8cbd710a463f54ae6f2ff9eb06c784c744)
// = 521de0a3ef2b7791666435a872ca9ec402ce886aff07bb4401de28bfdde4a13b
//
// hash of node 0
// = sha256(00 | 0000000000000000000000000000000000000000000000000000000000000000 | 521de0a3ef2b7791666435a872ca9ec402ce886aff07bb4401de28bfdde4a13b)
// = b843a96765fc40641227234e9f9a2736c2e0cdf8fb2dc54e358bb4fa29a61042
//
// hash of node 1
// = sha256(01 | sha256("a") | sha256("buzz"))
// = sha256(01 | ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb | 9fff3bcb10ca5e87b8109ccde9e9452012d634a005942afc46cf2b7fa307526a)
// = cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222
//
// root hash
// = sha256(00 | b843a96765fc40641227234e9f9a2736c2e0cdf8fb2dc54e358bb4fa29a61042 | cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222)
// = ae08c246d53a8ff3572a68d5bba4d610aaaa765e3ef535320c5653969aaa031b

#[cfg(test)]
mod tests {
    use {super::*, cw_std::MockStorage, hex_literal::hex};

    fn build_test_case(tree: &MerkleTree, store: &mut dyn Storage) -> StdResult<()> {
        tree.apply_raw(store, &Batch::from([
            (b"r".to_vec(), Op::Put(b"foo".to_vec())),
            (b"m".to_vec(), Op::Put(b"bar".to_vec())),
            (b"L".to_vec(), Op::Put(b"fuzz".to_vec())),
            (b"a".to_vec(), Op::Put(b"buzz".to_vec())),
        ]))
    }

    #[test]
    fn applying_batch() {
        let mut store = MockStorage::new();
        let tree = MerkleTree::default();
        build_test_case(&tree, &mut store).unwrap();

        // just check the root hash matches our calculation
        let root_hash = tree.root_hash(&store, None).unwrap().unwrap();
        assert_eq!(
            root_hash,
            Hash::from_slice(hex!("ae08c246d53a8ff3572a68d5bba4d610aaaa765e3ef535320c5653969aaa031b")),
        );
    }
}
