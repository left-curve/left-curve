use {
    crate::{Node, NodeKey, Proof},
    cw_std::{Batch, Hash, Item, Map, Set, StdResult, Storage},
};

pub const DEFAULT_VERSION_NAMESPACE: &str = "v";
pub const DEFAULT_NODE_NAMESPACE:    &str = "n";
pub const DEFAULT_ORPHAN_NAMESPACE:  &str = "o";

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

    /// Return the root hash at the given version.
    /// If version isn't specified, use the latest version.
    ///
    /// If the root node is not found at the given version, return None.
    /// There are two possible reasons that it's not found: either no data has
    /// ever been written to the tree yet, or the version is old and has been
    /// pruned.
    pub fn root_hash(&self, store: &dyn Storage, version: Option<u64>) -> StdResult<Option<Hash>> {
        let version = self.version_or_latest(store, version)?;
        let root_node_key = NodeKey::root(version);
        let root_node = self.nodes.may_load(store, &root_node_key)?;
        Ok(root_node.map(|node| node.hash()))
    }

    /// Generate Merkle proof for the a key at the given version. If the key
    /// exists in the tree, a membership proof is returned; otherwise,
    /// a non-membership proof is returned. If version is isn't specified, use
    /// the latest version.
    pub fn prove(
        &self,
        store:    &dyn Storage,
        key_hash: &Hash,
        version:  Option<u64>,
    ) -> StdResult<Proof> {
        todo!()
    }

    /// Apply a batch of ops to the tree. Each op is either 1) inserting a value
    /// at a key, or 2) deleting a key.
    /// Increments the version by 1, and recomputes the root hash.
    pub fn write_batch(&self, store: &mut dyn Storage, batch: Batch) -> StdResult<()> {
        todo!()
    }

    /// Delete nodes that no longer part of the tree since `up_to_version`
    /// (exclusive). If no `up_to_version` is provided then delete all orphans.
    pub fn prune(
        &self,
        store:         &mut dyn Storage,
        up_to_version: Option<u64>,
        limit:         Option<usize>,
    ) -> StdResult<()> {
        todo!()
    }

    fn version_or_latest(&self, store: &dyn Storage, maybe_version: Option<u64>) -> StdResult<u64> {
        match maybe_version {
            Some(v) => Ok(v),
            None => self.version.load(store),
        }
    }
}
