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

/// Describes what happens after applying ops (a slice of `HashedPair`) at a
/// node and its subtree.
#[derive(Debug)]
enum Outcome {
    /// No change happend to the node. Return `Some(node)` if the node existed,
    /// `None` if the node didn't exist.
    Unchanged(Option<Node>),
    /// The node has been changed. Return the changed node.
    Updated(Node),
    /// The node used to exist but has now been deleted.
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

        self.apply(store, batch)
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
        batch: Vec<(Hash, Op<Hash>)>,
    ) -> StdResult<(u64, Option<Hash>)> {
        let old_version = self.lateset_version(store)?;
        let old_root_node_key = NodeKey::root(old_version);
        let new_version = old_version + 1;
        let new_root_node_key = NodeKey::root(new_version);
        println!("applying new version = {new_version}");

        // recursively apply the ops, starting at the old root
        match self.apply_at(store, new_version, &old_root_node_key, batch)? {
            Outcome::Updated(new_root_node) => {
                self.version.save(store, &new_version)?;
                self.nodes.save(store, &new_root_node_key, &new_root_node)?;
                if old_version > 0 {
                    self.orphans.insert(store, (new_version, &old_root_node_key))?;
                }
                Ok((new_version, Some(new_root_node.hash())))
            },
            Outcome::Deleted => {
                self.version.save(store, &new_version)?;
                if old_version > 0 {
                    self.orphans.insert(store, (new_version, &old_root_node_key))?;
                }
                Ok((new_version, None))
            },
            Outcome::Unchanged(Some(old_root_node)) => {
                Ok((old_version, Some(old_root_node.hash())))
            },
            Outcome::Unchanged(None) => {
                Ok((old_version, None))
            },
        }
    }

    fn apply_at(
        &self,
        store:       &mut dyn Storage,
        new_version: u64,
        node_key:    &NodeKey,
        batch:       Vec<(Hash, Op<Hash>)>,
    ) -> StdResult<Outcome> {
        match self.nodes.may_load(store, &node_key)? {
            Some(Node::Leaf(leaf_node)) => {
                self.apply_at_leaf(store, new_version, node_key, leaf_node, batch)
            },
            Some(Node::Internal(internal_node)) => {
                self.apply_at_internal( store, new_version, node_key, internal_node, batch)
            },
            None => {
                self.create_subtree(store, new_version, node_key, only_inserts(batch), None)
            },
        }
    }

    fn apply_at_internal(
        &self,
        store:             &mut dyn Storage,
        new_version:       u64,
        node_key:          &NodeKey,
        mut internal_node: InternalNode,
        batch:             Vec<(Hash, Op<Hash>)>,
    ) -> StdResult<Outcome> {
        // split the batch into two, one for left child, one for right
        let (batch_for_left, batch_for_right) = partition_batch(batch, node_key);

        // apply at the two children, respectively
        let left_outcome = self.apply_at_child(
            store,
            new_version,
            &node_key,
            true,
            internal_node.left_child.as_ref(),
            batch_for_left,
        )?;
        let right_outcome = self.apply_at_child(
            store,
            new_version,
            &node_key,
            false,
            internal_node.right_child.as_ref(),
            batch_for_right,
        )?;

        match (left_outcome, right_outcome) {
            // both children are deleted or never existed. delete this node as well
            (Outcome::Deleted | Outcome::Unchanged(None), Outcome::Deleted | Outcome::Unchanged(None)) => {
                Ok(Outcome::Deleted)
            },
            // neither children is changed. this node is unchanged as well
            (Outcome::Unchanged(_), Outcome::Unchanged(_)) => {
                Ok(Outcome::Unchanged(Some(Node::Internal(internal_node))))
            },
            // left child is a leaf, right child is deleted.
            // delete the current internal node and move left child up.
            (Outcome::Updated(left) | Outcome::Unchanged(Some(left)), Outcome::Deleted | Outcome::Unchanged(None)) if left.is_leaf() => {
                Ok(Outcome::Updated(left))
            },
            // left child is deleted, right child is a leaf.
            // delete the current internal node and move right child up.
            (Outcome::Deleted | Outcome::Unchanged(None), Outcome::Updated(right) | Outcome::Unchanged(Some(right))) if right.is_leaf() => {
                Ok(Outcome::Updated(right))
            },
            // at least one child is updated and the path can't be collapsed.
            // update the currenct node and return
            (left, right) => {
                internal_node.left_child = match left {
                    Outcome::Updated(child_node) => {
                        Some(Child {
                            version: new_version,
                            hash: child_node.hash(),
                        })
                    },
                    Outcome::Deleted => None,
                    Outcome::Unchanged(_) => internal_node.left_child,
                };

                internal_node.right_child = match right {
                    Outcome::Updated(child_node) => {
                        Some(Child {
                            version: new_version,
                            hash: child_node.hash(),
                        })
                    },
                    Outcome::Deleted => None,
                    Outcome::Unchanged(_) => internal_node.right_child,
                };

                Ok(Outcome::Updated(Node::Internal(internal_node)))
            },
        }
    }

    #[inline]
    fn apply_at_child(
        &self,
        store:       &mut dyn Storage,
        new_version: u64,
        node_key:    &NodeKey,
        is_left:     bool,
        child:       Option<&Child>,
        batch:       Vec<(Hash, Op<Hash>)>,
    ) -> StdResult<Outcome> {
        match (child, batch.len()) {
            // child exists, but there is no op to apply
            (Some(Child { version, .. }), 0) => {
                let child_node_key = node_key.child_at_version(is_left, *version);
                let child_node = self.nodes.load(store, &child_node_key)?;
                Ok(Outcome::Unchanged(Some(child_node)))
            },
            // child doesn't exist, and there is no op to apply
            (None, 0) => {
                Ok(Outcome::Unchanged(None))
            },
            // child exists, and there are ops to apply
            (Some(Child { version, .. }), _) => {
                let mut child_node_key = node_key.child_at_version(is_left, *version);
                let outcome = self.apply_at(store, new_version, &child_node_key, batch)?;
                // if the child has been updated, save the updated node
                if let Outcome::Updated(new_child_node) = &outcome {
                    child_node_key.version = new_version;
                    self.nodes.save(store, &child_node_key, &new_child_node)?;
                }
                // if the child has been deleted or updated, mark it as orphaned
                if let Outcome::Deleted | Outcome::Updated(_) = &outcome {
                    self.orphans.insert(store, (new_version, &child_node_key))?;
                }
                Ok(outcome)
            },
            // child doesn't exist, but there are ops to apply
            (None, _) => {
                let child_node_key = node_key.child_at_version(is_left, new_version);
                self.create_subtree(store, new_version, &child_node_key, only_inserts(batch), None)
            },
        }
    }

    fn apply_at_leaf(
        &self,
        store:         &mut dyn Storage,
        new_version:   u64,
        node_key:      &NodeKey,
        mut leaf_node: LeafNode,
        batch:         Vec<(Hash, Op<Hash>)>,
    ) -> StdResult<Outcome> {
        if batch.len() == 1 {
            let (key_hash, op) = only_item(batch);
            return match (key_hash == leaf_node.key_hash, op) {
                (true, Op::Delete) => {
                    Ok(Outcome::Deleted)
                },
                (false, Op::Delete) => {
                    Ok(Outcome::Unchanged(Some(Node::Leaf(leaf_node))))
                },
                (true, Op::Insert(value_hash)) => {
                    leaf_node.value_hash = value_hash;
                    Ok(Outcome::Updated(Node::Leaf(leaf_node)))
                },
                (false, Op::Insert(value_hash)) => {
                    self.create_subtree(
                        store,
                        new_version,
                        node_key,
                        vec![(key_hash, value_hash)],
                        Some(leaf_node),
                    )
                },
            }
        }

        // we don't need to worry about the existing leaf if it will be
        // overwritten or deleted
        let existing_leaf = if batch.iter().any(|(k, _)| *k == leaf_node.key_hash) {
            None
        } else {
            Some(leaf_node)
        };

        self.create_subtree(store, new_version, node_key, only_inserts(batch), existing_leaf)
    }

    fn create_subtree(
        &self,
        store:         &mut dyn Storage,
        new_version:   u64,
        node_key:      &NodeKey,
        batch:         Vec<(Hash, Hash)>,
        existing_leaf: Option<LeafNode>,
    ) -> StdResult<Outcome> {
        let new_node = match (batch.len(), existing_leaf) {
            (0, None) => {
                return Ok(Outcome::Unchanged(None));
            },
            (0, Some(leaf_node)) => {
                Node::Leaf(leaf_node)
            }
            (1, None) => {
                let (key_hash, value_hash) = only_item(batch);
                Node::Leaf(LeafNode::new(key_hash, value_hash))
            },
            (_, existing_leaf) => {
                let (batch_for_left, batch_for_right) = partition_batch(batch, node_key);
                let (leaf_for_left, leaf_for_right) = partition_leaf(existing_leaf, node_key);
                let left_outcome = self.create_subtree(
                    store,
                    new_version,
                    &node_key.child_at_version(true, new_version),
                    batch_for_left,
                    leaf_for_left,
                )?;
                let right_outcome = self.create_subtree(
                    store,
                    new_version,
                    &node_key.child_at_version(false, new_version),
                    batch_for_right,
                    leaf_for_right,
                )?;
                Node::Internal(InternalNode {
                    left_child: into_child(new_version, left_outcome),
                    right_child: into_child(new_version, right_outcome),
                })
            },
        };

        self.nodes.save(store, node_key, &new_node)?;

        Ok(Outcome::Updated(new_node))
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
fn partition_batch<T>(mut batch: Vec<(Hash, T)>, nk: &NodeKey) -> (Vec<(Hash, T)>, Vec<(Hash, T)>) {
    let partition_point = batch.partition_point(|(key_hash, _)| {
        bit_at_index(key_hash, nk.bits.num_bits) == 0
    });
    let right = batch.split_off(partition_point);
    (batch, right)
}

#[inline]
fn partition_leaf(leaf: Option<LeafNode>, nk: &NodeKey) -> (Option<LeafNode>, Option<LeafNode>) {
    if let Some(leaf) = leaf {
        let bit = bit_at_index(&leaf.key_hash, nk.bits.num_bits);
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

/// Given a batch, which may contain both inserts and deletes, remove all the
/// deletes, only keep the inserts.
#[inline]
fn only_inserts(batch: Vec<(Hash, Op<Hash>)>) -> Vec<(Hash, Hash)> {
    batch
        .into_iter()
        .filter_map(|(key_hash, op)| {
            if let Op::Insert(value_hash) = op {
                Some((key_hash, value_hash))
            } else {
                None
            }
        })
        .collect()
}

/// Consume a vector, assert it has exactly item, return this item by value.
#[inline]
fn only_item<T>(mut vec: Vec<T>) -> T {
    debug_assert_eq!(vec.len(), 1);
    vec.pop().unwrap()
}

/// Get the i-th bit without having to cast the byte slice to BitArray (which
/// involves some copying).
#[inline]
fn bit_at_index(bytes: &[u8], index: usize) -> u8 {
    let (quotient, remainder) = (index / 8, index % 8);
    let byte = bytes[quotient];
    (byte >> (7 - remainder)) & 0b1
}

#[inline]
fn hash_of(child: Option<Child>) -> Option<Hash> {
    child.map(|child| child.hash)
}

#[inline]
fn into_child(version: u64, outcome: Outcome) -> Option<Child> {
    match outcome {
        Outcome::Updated(node) => {
            Some(Child {
                version,
                hash: node.hash(),
            })
        },
        Outcome::Unchanged(None) => None,
        _ => unreachable!("invalid outcome when building subtree: {outcome:?}"),
    }
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
        let (store, version, root_hash) = build_test_case().unwrap();
        assert_eq!(version, 1);
        assert_eq!(root_hash, Some(HASH_ROOT));
        assert!(TREE.orphans.range(&store, None, None, Order::Ascending).next().is_none());
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

    // no-op is when the batch contains entirely of overwrites of values by the
    // same value, or deletes of non-existing keys. the version number shouldn't
    // be incremented and root hash shouldn't be changed.
    #[test]
    fn no_ops() {
        let (mut store, _, _) = build_test_case().unwrap();

        let (new_version, new_root_hash) = TREE.apply_raw(&mut store, &Batch::from([
            // overwriting keys with the same keys
            (b"r".to_vec(), Op::Insert(b"foo".to_vec())),
            (b"m".to_vec(), Op::Insert(b"bar".to_vec())),
            (b"L".to_vec(), Op::Insert(b"fuzz".to_vec())),
            (b"a".to_vec(), Op::Insert(b"buzz".to_vec())),
            // deleting non-existing keys
            (b"larry".to_vec(), Op::Delete), // 00001101...
            (b"trump".to_vec(), Op::Delete), // 10100110...
            (b"biden".to_vec(), Op::Delete), // 00000110...
        ]))
        .unwrap();
        assert_eq!(new_version, 1);
        assert_eq!(TREE.lateset_version(&store).unwrap(), 1);
        assert_eq!(new_root_hash, Some(HASH_ROOT));

        // make sure that no node has been marked as orphaned
        assert!(TREE.orphans.range(&store, None, None, Order::Ascending).next().is_none());

        // make sure no node of version 2 has been written
        for item in TREE.nodes.keys(&store, None, None, Order::Ascending) {
            let node_key = item.unwrap();
            assert_eq!(node_key.version, 1);
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
