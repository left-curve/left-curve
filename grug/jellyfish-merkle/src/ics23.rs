use {
    crate::{
        BitArray, INTERNAL_NODE_HASH_PREFIX, InternalNode, LEAF_NODE_HASH_PERFIX, MerkleTree, Node,
        ROOT_BITS,
    },
    grug_types::{Hash256, Order, StdResult, Storage},
    ics23::{HashOp, InnerOp, InnerSpec, LeafOp, LengthOp, ProofSpec},
    std::sync::LazyLock,
};

/// ICS-23 proof spec of Grug's Jellyfish Merkle Tree.
///
/// This value requires dynamic allocation, so can't be declared as a constant;
/// we use a `LazyLock` instead.
///
/// Note: if we change the hash function at some point (we're considering BLAKE3
/// over SHA-256), this needs to be updated.
pub static ICS23_PROOF_SPEC: LazyLock<ProofSpec> = LazyLock::new(|| ProofSpec {
    leaf_spec: Some(LeafOp {
        hash: HashOp::Sha256.into(),
        prehash_key: HashOp::Sha256.into(),
        prehash_value: HashOp::Sha256.into(),
        length: LengthOp::NoPrefix.into(),
        prefix: LEAF_NODE_HASH_PERFIX.to_vec(),
    }),
    inner_spec: Some(InnerSpec {
        child_order: vec![0, 1],
        child_size: Hash256::LENGTH as _,
        min_prefix_length: INTERNAL_NODE_HASH_PREFIX.len() as _,
        max_prefix_length: INTERNAL_NODE_HASH_PREFIX.len() as _,
        empty_child: Hash256::ZERO.to_vec(),
        hash: HashOp::Sha256.into(),
    }),
    max_depth: 256,
    min_depth: 0,
    prehash_key_before_comparison: true,
});

impl MerkleTree<'_> {
    /// Traverse the tree, find the leaf node containing the key hash, and
    /// return the ICS-23 path (the list of `InnerOp`'s) that can prove this
    /// key's existence.
    ///
    /// ## Panics
    ///
    /// Panics if the key is not found. The caller must ensure the key exists
    /// before calling. This is typically done by querying the state storage
    /// first.
    pub fn ics23_prove_existence(
        &self,
        storage: &dyn Storage,
        version: u64,
        key_hash: Hash256,
    ) -> StdResult<Vec<InnerOp>> {
        let mut bits = ROOT_BITS;
        let bitarray = BitArray::from_bytes(&key_hash);
        let mut iter = bitarray.range(None, None, Order::Ascending);
        let mut node = self.nodes.load(storage, (version, &bits))?;
        let mut path = vec![];

        loop {
            match node {
                Node::Leaf(leaf) => {
                    assert_eq!(leaf.key_hash, key_hash, "target key hash not found");
                    break;
                },
                Node::Internal(InternalNode {
                    left_child,
                    right_child,
                }) => match (iter.next(), left_child, right_child) {
                    (Some(0), Some(child), sibling) => {
                        bits.push(0);
                        node = self.nodes.load(storage, (child.version, &bits))?;
                        path.push(InnerOp {
                            // Not sure why we have to include the `HashOp` here
                            // when it's already in the `ProofSpec`.
                            hash: ICS23_PROOF_SPEC.inner_spec.as_ref().unwrap().hash,
                            prefix: INTERNAL_NODE_HASH_PREFIX.to_vec(),
                            suffix: sibling.map(|c| c.hash).unwrap_or(Hash256::ZERO).to_vec(),
                        });
                    },
                    (Some(1), sibling, Some(child)) => {
                        bits.push(1);
                        node = self.nodes.load(storage, (child.version, &bits))?;
                        path.push(InnerOp {
                            hash: ICS23_PROOF_SPEC.inner_spec.as_ref().unwrap().hash,
                            prefix: [
                                INTERNAL_NODE_HASH_PREFIX,
                                sibling.map(|c| c.hash).unwrap_or(Hash256::ZERO).as_ref(),
                            ]
                            .concat(),
                            suffix: vec![],
                        })
                    },
                    _ => unreachable!("target key hash not found"),
                },
            }
        }

        // The path goes from bottom up, so needs to be reversed.
        path.reverse();

        Ok(path)
    }
}
