use {
    crate::{
        BitArray, Child, InternalNode, LeafNode, MembershipProof, Node, NodeKey,
        NonMembershipProof, Proof, ProofNode,
    },
    cw_std::{hash, Batch, Hash, Item, Map, Op, Order, Set, StdResult, Storage},
};

pub const DEFAULT_VERSION_NAMESPACE: &str  = "v";
pub const DEFAULT_NODE_NAMESPACE:    &str  = "n";
pub const DEFAULT_ORPHAN_NAMESPACE:  &str  = "o";

/// Describe a database operation, which can be either inserting a value, or
/// deleting the value, under the given key. Both the key and value are hashed.
pub type HashedPair = (Hash, Op<Hash>);

/// Describes the outcome of applying ops at a node.
enum OpResponse {
    /// The node has not been changed. This can be the result of overwriting a
    /// value with the same value, or deleting a key that doesn't exist in the
    /// first place.
    /// - `Unchanged(Some(_))` means the node exists, and after applying the ops
    ///   it hasn't been changed.
    /// - `Unchanged(None)` means the node didn't exist, and after applying the
    ///   ops it still doesn't exist.
    Unchanged(Option<Node>),
    /// The node didn't exist, but has now been created; or, it used to exist,
    /// and has now been modified.
    Updated(Node),
    /// The node used to exist, but no longer exists after applying the ops.
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
        Self::new_default()
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

    /// Create a new Merkle tree with the default namespaces.
    ///
    /// The `Default` feature does not allow declaring constants, so use this.
    pub const fn new_default() -> Self {
        Self::new(DEFAULT_VERSION_NAMESPACE, DEFAULT_NODE_NAMESPACE, DEFAULT_ORPHAN_NAMESPACE)
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

    /// Apply a batch of ops to the tree. Return the new version and root hash.
    ///
    /// If the tree isn't changed, the version isn't incremented.
    ///
    /// If the tree becomes empty after applying the ops, `None` is returned as
    /// the new root.
    ///
    /// This function takes a batch where both the keys and values are prehashes.
    /// If you already have them hashed and sorted ascendingly by the key hashes,
    /// use `apply` instead.
    pub fn apply_raw(
        &self,
        store: &mut dyn Storage,
        batch: &Batch,
    ) -> StdResult<(u64, Option<Hash>)> {
        // hash the keys and values
        let mut batch: Vec<_> = batch.iter().map(|(k, op)| (hash(k), op.as_ref().map(hash))).collect();

        // sort by key hashes ascendingly
        batch.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));

        self.apply(store, &batch)
    }

    /// Apply a batch of ops to the tree. Return the new version and root hash.
    ///
    /// If the tree isn't changed, the version isn't incremented.
    ///
    /// If the tree becomes empty after applying the ops, `None` is returned as
    /// the new root.
    ///
    /// This function takes a `HashedBatch` where both the keys and values are
    /// hashed, and sorted ascendingly by the key hashes. If you have a batch
    /// of prehashes, use `apply_raw` instead.
    pub fn apply(
        &self,
        store: &mut dyn Storage,
        batch: &[HashedPair],
    ) -> StdResult<(u64, Option<Hash>)> {
        let old_version = self.lateset_version(store)?;
        let new_version = old_version + 1;

        // recursively apply the ops, starting at the old root
        match self.apply_at(store, new_version, NodeKey::root(old_version), batch, None)? {
            OpResponse::Updated(new_root_node) => {
                // root hash has been changed. increment the version
                self.version.save(store, &new_version)?;
                Ok((new_version, Some(new_root_node.hash())))
            },
            OpResponse::Deleted => {
                self.version.save(store, &new_version)?;
                Ok((new_version, None))
            }
            OpResponse::Unchanged(old_root_node) => {
                // do not increment the version if the tree hasn't been changed
                Ok((old_version, old_root_node.map(|node| node.hash())))
            },
        }
    }

    fn apply_at(
        &self,
        store:         &mut dyn Storage,
        new_version:   u64,
        node_key:      NodeKey,
        batch:         &[HashedPair],
        existing_leaf: Option<LeafNode>,
    ) -> StdResult<OpResponse> {
        match self.nodes.may_load(store, &node_key)? {
            Some(Node::Leaf(leaf_node)) => {
                // we can't run into leaf nodes twice during the same insertion.
                // if our current node is a leaf, this means the existing leaf
                // must be None.
                debug_assert!(existing_leaf.is_none(), "encountered leaves twice");
                self.apply_at_leaf(store, new_version, node_key, leaf_node, batch)
            },
            Some(Node::Internal(internal_node)) => {
                self.apply_at_internal( store, new_version, node_key, Some(internal_node), batch, existing_leaf)
            },
            None => {
                self.apply_at_null(store, new_version, node_key, batch, existing_leaf)
            },
        }
    }

    fn apply_at_internal(
        &self,
        store:         &mut dyn Storage,
        new_version:   u64,
        mut node_key:  NodeKey,
        // Some if we're working with an existing internal node;
        // None if the internal node doesn't exist yet.
        internal_node: Option<InternalNode>,
        batch:         &[HashedPair],
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
            new_version,
            &node_key,
            true,
            internal_node.as_ref().and_then(|node| node.left_child.as_ref()),
            batch_for_left,
            existing_leaf_for_left,
        )?;
        let right_response = self.apply_at_child(
            store,
            new_version,
            &node_key,
            false,
            internal_node.as_ref().and_then(|node| node.right_child.as_ref()),
            batch_for_right,
            existing_leaf_for_right,
        )?;

        match (left_response, right_response) {
            // neither children has been changed. this node is unchanged as well
            (OpResponse::Unchanged(_), OpResponse::Unchanged(_)) => {
                Ok(OpResponse::Unchanged(internal_node.map(Node::Internal)))
            },
            // neither children exists any more. this internal node can be deleted
            (OpResponse::Deleted | OpResponse::Unchanged(None), OpResponse::Deleted | OpResponse::Unchanged(None)) => {
                self.orphans.insert(store, (new_version, &node_key))?;
                Ok(OpResponse::Deleted)
            },
            // left child does not exist, right child is a leaf.
            // in this case, the internal node can be deleted, and the right
            // child be moved up one level. we call this "collapsing the path".
            (OpResponse::Deleted | OpResponse::Unchanged(None), OpResponse::Unchanged(Some(node)) | OpResponse::Updated(node)) if node.is_leaf() => {
                self.orphans.insert(store, (new_version, &node_key))?;
                Ok(OpResponse::Updated(node))
            },
            // left child is a leaf, right node no longer exists.
            // path can be collapsed (same as above).
            (OpResponse::Unchanged(Some(node)) | OpResponse::Updated(node), OpResponse::Deleted | OpResponse::Unchanged(None)) if node.is_leaf() => {
                self.orphans.insert(store, (new_version, &node_key))?;
                Ok(OpResponse::Updated(node))
            },
            // at least one child is updated and the path can't be collapsed.
            // update the currenct node and return
            (left, right) => {
                self.orphans.insert(store, (new_version, &node_key))?;

                let mut internal_node = internal_node.unwrap_or_else(InternalNode::new_childless);
                internal_node.left_child = match left {
                    OpResponse::Updated(child_node) => {
                        Some(Child {
                            version: new_version,
                            hash: child_node.hash(),
                        })
                    },
                    OpResponse::Deleted => None,
                    OpResponse::Unchanged(_) => internal_node.left_child,
                };
                internal_node.right_child = match right {
                    OpResponse::Updated(child_node) => {
                        Some(Child {
                            version: new_version,
                            hash: child_node.hash(),
                        })
                    },
                    OpResponse::Deleted => None,
                    OpResponse::Unchanged(_) => internal_node.right_child,
                };
                let node = Node::Internal(internal_node);

                node_key.version = new_version;
                self.nodes.save(store, &node_key, &node)?;

                Ok(OpResponse::Updated(node))
            },
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn apply_at_child(
        &self,
        store:         &mut dyn Storage,
        new_version:   u64,
        node_key:      &NodeKey,
        is_left:       bool,
        child:         Option<&Child>,
        batch:         &[HashedPair],
        existing_leaf: Option<LeafNode>,
    ) -> StdResult<OpResponse> {
        // if the batch is non-empty OR there's an existing leaf to be inserted,
        // then we recursively apply at the child node
        if !batch.is_empty() || existing_leaf.is_some() {
            let child_version = child.map(|c| c.version).unwrap_or(new_version);
            let child_node_key = node_key.child_at_version(is_left, child_version);
            return self.apply_at(store, new_version, child_node_key, batch, existing_leaf);
        }

        // the batch is empty AND there isn't an existing leaf to be inserted.
        // there's nothing to be done. we end the recursion here.
        if let Some(child) = child {
            let child_node_key = node_key.child_at_version(is_left, child.version);
            let child_node = self.nodes.load(store, &child_node_key)?;
            return Ok(OpResponse::Unchanged(Some(child_node)));
        }

        Ok(OpResponse::Unchanged(None))
    }

    fn apply_at_leaf(
        &self,
        store:         &mut dyn Storage,
        new_version:   u64,
        mut node_key:  NodeKey,
        mut leaf_node: LeafNode,
        batch:         &[HashedPair],
    ) -> StdResult<OpResponse> {
        // if there is only one op AND the key matches exactly the leaf node's
        // key, then we have found the correct node, apply the op right here.
        if batch.len() == 1 {
            let (bits, op) = &batch[0];
            if *bits == leaf_node.key_hash {
                return match op {
                    Op::Insert(value_hash) => {
                        if leaf_node.value_hash == *value_hash {
                            // overwriting the value with the same value
                            // node is unchanged
                            Ok(OpResponse::Unchanged(Some(Node::Leaf(leaf_node))))
                        } else {
                            // overwriting with a different value
                            // node is updated, save the new node
                            node_key.version = new_version;
                            leaf_node.value_hash = value_hash.clone();
                            let node = Node::Leaf(leaf_node);
                            self.nodes.save(store, &node_key, &node)?;
                            Ok(OpResponse::Updated(node))
                        }
                    },
                    Op::Delete => {
                        // node is deleted, mark it as orphaned
                        self.orphans.insert(store, (new_version, &node_key))?;
                        Ok(OpResponse::Deleted)
                    }
                };
            }
        }

        // now we know we can't apply the op right at this leaf node, either
        // because there are more than one ops in the batch, or there is only
        // one but it doesn't match the current leaf. either way, we have to
        // turn the current node into an internal node and apply the batch at
        // its children.
        let existing_leaf = if batch.iter().any(|(k, _)| *k == leaf_node.key_hash) {
            // if the existing leaf will be overwritten by the batch, then we
            // don't need to worry about it.
            None
        } else {
            Some(leaf_node)
        };
        self.apply_at_internal(store, new_version, node_key, None, batch, existing_leaf)
    }

    fn apply_at_null(
        &self,
        store:         &mut dyn Storage,
        new_version:   u64,
        mut node_key:  NodeKey,
        batch:         &[HashedPair],
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
                    Op::Insert(value_hash) => {
                        node_key.version = new_version;
                        let node = Node::Leaf(LeafNode::new(key_hash.clone(), value_hash.clone()));
                        self.nodes.save(store, &node_key, &node)?;
                        OpResponse::Updated(node)
                    },
                    Op::Delete => OpResponse::Unchanged(None),
                }
            }),
            // there are more than one op to do.
            // regardless of whether there's an existing leaf, create an empty
            // internal node and apply the batch at this internal node.
            (_, existing_leaf) => {
                self.apply_at_internal(store, new_version, node_key, None, batch, existing_leaf)
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
            Ok(Proof::NonMembership(NonMembershipProof { node, sibling_hashes }))
        } else {
            Ok(Proof::Membership(MembershipProof { sibling_hashes }))
        }
    }

    /// Delete nodes that are no longer part of the tree as of `up_to_version`.
    /// If no `up_to_version` is provided then delete all orphans.
    pub fn prune(&self, _store: &mut dyn Storage, _up_to_version: Option<u64>) -> StdResult<()> {
        // we should first implement a `range_remove` method on Storage trait
        todo!()
    }
}

#[inline]
fn partition_batch(batch: &[HashedPair], depth: usize) -> (&[HashedPair], &[HashedPair]) {
    let partition_point = batch.partition_point(|(key_hash, _)| {
        bit_at_index(key_hash, depth) == 0
    });
    (&batch[..partition_point], &batch[partition_point..])
}

#[inline]
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
#[inline]
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
//        010  (011)
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
    use {super::*, cw_std::MockStorage, hex_literal::hex, test_case::test_case};

    const TREE: MerkleTree = MerkleTree::new_default();

    const HASH_ROOT: Hash = Hash::from_slice(hex!("ae08c246d53a8ff3572a68d5bba4d610aaaa765e3ef535320c5653969aaa031b"));
    const HASH_0:    Hash = Hash::from_slice(hex!("b843a96765fc40641227234e9f9a2736c2e0cdf8fb2dc54e358bb4fa29a61042"));
    const HASH_1:    Hash = Hash::from_slice(hex!("cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222"));
    const HASH_01:   Hash = Hash::from_slice(hex!("521de0a3ef2b7791666435a872ca9ec402ce886aff07bb4401de28bfdde4a13b"));
    const HASH_010:  Hash = Hash::from_slice(hex!("c8348e9a7a327e8b76e97096c362a1f87071ee4108b565d1f409529c189cb684"));
    const HASH_011:  Hash = Hash::from_slice(hex!("e104e2bcf24027af737c021033cb9d8cbd710a463f54ae6f2ff9eb06c784c744"));
    const HASH_0110: Hash = Hash::from_slice(hex!("fd34e3f8d9840e7f6d6f639435b6f9b67732fc5e3d5288e268021aeab873f280"));
    const HASH_0111: Hash = Hash::from_slice(hex!("412341380b1e171077dd9da9af936ae2126ede2dd91dc5acb0f77363d46eb76b"));
    const HASH_M:    Hash = Hash::from_slice(hex!("62c66a7a5dd70c3146618063c344e531e6d4b59e379808443ce962b3abd63c5a"));
    const HASH_BAR:  Hash = Hash::from_slice(hex!("fcde2b2edba56bf408601fb721fe9b5c338d10ee429ea04fae5511b68fbf8fb9"));

    fn build_test_case() -> StdResult<(MockStorage, u64, Option<Hash>)> {
        let mut store = MockStorage::new();
        let (version, root_hash) = TREE.apply_raw(&mut store, &Batch::from([
            (b"r".to_vec(), Op::Insert(b"foo".to_vec())),
            (b"m".to_vec(), Op::Insert(b"bar".to_vec())),
            (b"L".to_vec(), Op::Insert(b"fuzz".to_vec())),
            (b"a".to_vec(), Op::Insert(b"buzz".to_vec())),
        ]))?;
        Ok((store, version, root_hash))
    }

    #[test]
    fn applying_initial_batch() {
        let (_, version, root_hash) = build_test_case().unwrap();
        // if root hash matches our expected value then we consider it a success
        assert_eq!(version, 1);
        assert_eq!(root_hash, Some(HASH_ROOT));
    }

    // delete the leaves 010 and 0110. this should cause the leaf 0111 be moved
    // up to bit path `0`. the result tree is:
    //
    //           root
    //         ┌──┴──┐
    //         0     1
    //
    // hash of node 0
    // = 412341380b1e171077dd9da9af936ae2126ede2dd91dc5acb0f77363d46eb76b
    // (the same as that of node 0111 of the last version)
    //
    // hash of node 1
    // = cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222
    // (unchanged)
    //
    // root hash
    // = sha256(00 | 412341380b1e171077dd9da9af936ae2126ede2dd91dc5acb0f77363d46eb76b | cb640e68682628445a3e0713fafe91b9cefe4f81c2337e9d3df201d81ae70222)
    // = b3e4002b2d95d57ab44bbf64c8cfb04904c02fb2df9c859a75d82b02fd087dbf
    #[test]
    fn collapsing_path() {
        let (mut store, _, _) = build_test_case().unwrap();
        let (new_version, new_root_hash) = TREE.apply_raw(&mut store, &Batch::from([
            (b"r".to_vec(), Op::Delete),
            (b"m".to_vec(), Op::Delete),
        ]))
        .unwrap();
        assert_eq!(new_version, 2);
        assert_eq!(new_root_hash, Some(Hash::from_slice(hex!("b3e4002b2d95d57ab44bbf64c8cfb04904c02fb2df9c859a75d82b02fd087dbf"))));
    }

    // try deleting every single node. the function should return None as the
    // new root hash. see that nodes have been properly marked as orphaned.
    #[test]
    fn deleting_all_nodes() {
        let (mut store, _, _) = build_test_case().unwrap();

        // check that new root hash is None
        let (new_version, new_root_hash) = TREE.apply_raw(&mut store, &Batch::from([
            (b"r".to_vec(), Op::Delete),
            (b"m".to_vec(), Op::Delete),
            (b"L".to_vec(), Op::Delete),
            (b"a".to_vec(), Op::Delete),
        ]))
        .unwrap();
        assert_eq!(new_version, 2);
        assert!(new_root_hash.is_none());

        // check that every node has been marked as orphaned
        for item in TREE.nodes.keys(&store, None, None, Order::Ascending) {
            let node_key = item.unwrap();
            assert_eq!(node_key.version, 1);
            assert!(TREE.orphans.has(&store, (2, &node_key)));
        }
    }

    #[test_case(
        "r",
        Proof::Membership(MembershipProof {
            sibling_hashes: vec![
                Some(HASH_011),
                None,
                Some(HASH_1),
            ],
        });
        "proving membership of r"
    )]
    #[test_case(
        "m",
        Proof::Membership(MembershipProof {
            sibling_hashes: vec![
                Some(HASH_0111),
                Some(HASH_010),
                None,
                Some(HASH_1),
            ],
        });
        "proving membership of m"
    )]
    #[test_case(
        "L",
        Proof::Membership(MembershipProof {
            sibling_hashes: vec![
                Some(HASH_0110),
                Some(HASH_010),
                None,
                Some(HASH_1),
            ],
        });
        "proving membership of L"
    )]
    #[test_case(
        "a",
        Proof::Membership(MembershipProof {
            sibling_hashes: vec![Some(HASH_0)],
        });
        "proving membership of a"
    )]
    #[test_case(
        "b", // sha256("b") = 0011... node 0 doesn't have a left child
        Proof::NonMembership(NonMembershipProof {
            node: ProofNode::Internal {
                left_hash:  None,
                right_hash: Some(HASH_01),
            },
            sibling_hashes: vec![Some(HASH_1)],
        });
        "proving non-membership of b"
    )]
    #[test_case(
        "o", // sha256("o") = 011001... there's a leaf 0110 ("m") which doesn't match key
        Proof::NonMembership(NonMembershipProof {
            node: ProofNode::Leaf {
                key_hash:   HASH_M,
                value_hash: HASH_BAR,
            },
            sibling_hashes: vec![
                Some(HASH_0111),
                Some(HASH_010),
                None,
                Some(HASH_1),
            ],
        });
        "proving non-membership of o"
    )]
    fn proving(key: &str, proof: Proof) {
        let (store, _, _) = build_test_case().unwrap();
        assert_eq!(TREE.prove(&store, &hash(key.as_bytes()), None).unwrap(), proof);
    }
}
