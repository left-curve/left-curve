use {
    crate::{BitArray, Node, NodeKey, Proof},
    cw_std::{Batch, Hash, Item, Map, Op, Set, StdResult, Storage},
};

pub const DEFAULT_VERSION_NAMESPACE: &str = "v";
pub const DEFAULT_NODE_NAMESPACE:    &str = "n";
pub const DEFAULT_ORPHAN_NAMESPACE:  &str = "o";

enum OpResponse {
    Updated(Node),
    Deleted,
    Unchanged,
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
pub struct Tree<'a> {
    version: Item<'a, u64>,
    nodes:   Map<'a, &'a NodeKey, Node>,
    orphans: Set<'a, (u64, &'a NodeKey)>,
}

impl<'a> Default for Tree<'a> {
    fn default() -> Self {
        Self::new(DEFAULT_VERSION_NAMESPACE, DEFAULT_NODE_NAMESPACE, DEFAULT_ORPHAN_NAMESPACE)
    }
}

impl<'a> Tree<'a> {
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
    pub fn apply(&self, store: &mut dyn Storage, batch: Batch) -> StdResult<()> {
        let old_version = self.version.may_load(store)?.unwrap_or(0);
        let old_root_node_key = NodeKey::root(old_version);
        let new_version = old_version + 1;
        let new_root_node_key = NodeKey::root(new_version);

        // convert the raw keys to bitarrays
        let batch = batch.into_iter().map(|(k, op)| (BitArray::from(k.as_slice()), k, op));

        // recursively apply the ops, starting at the old root
        match self.apply_at(store, new_version, &old_root_node_key, None, batch)? {
            OpResponse::Updated(new_root_node) => {
                // the root node has been updated. save this new node.
                self.nodes.save(store, &new_root_node_key, &new_root_node)?;
            },
            OpResponse::Unchanged => {
                // new node is the same as the old one. copy the old node.
                // except if the old version is zero, in which case the old root
                // node doesn't exist.
                if old_version > 0 {
                    let old_root_node = self.nodes.load(store, &old_root_node_key)?;
                    self.nodes.save(store, &new_root_node_key, &old_root_node)?;
                }
            }
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
        _store:            &mut dyn Storage,
        _version:          u64,
        _current_node_key: &NodeKey,
        _current_node:     Option<Node>,
        _batch:            impl Iterator<Item = (BitArray, Vec<u8>, Op)>,
    ) -> StdResult<OpResponse> {
        todo!()
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
