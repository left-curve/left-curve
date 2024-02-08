use {
    crate::{hash_internal_node, hash_leaf_node, hash_of, BitArray, InternalNode, LeafNode, Node},
    cw_std::{cw_serde, Hash},
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum ProofError {
    #[error("incorrect proof type, expect {expect}, got {actual}")]
    IncorrectProofType {
        expect: &'static str,
        actual: &'static str,
    },

    #[error("root hash mismatch! computed: {computed}, actual: {actual}")]
    RootHashMismatch {
        computed: Hash,
        actual:   Hash,
    },

    // TODO: add more details to the error message?
    #[error("expecting child to not exist but it exists")]
    UnexpectedChild,

    // TODO: add more details to the error message?
    #[error("expecting bitarrays to share a common prefix but they do not")]
    NotCommonPrefix,
}

#[cw_serde]
pub enum Proof {
    Membership {
        sibling_hashes: Vec<Option<Hash>>,
    },
    NonMembership {
        node: Node,
        sibling_hashes: Vec<Option<Hash>>,
    },
}

pub fn verify_membership(
    root_hash:  &Hash,
    key_hash:   &Hash,
    value_hash: &Hash,
    proof:      &Proof,
) -> Result<(), ProofError> {
    let Proof::Membership { sibling_hashes } = proof else {
        return Err(ProofError::IncorrectProofType {
            expect: "membership",
            actual: "non-membership",
        });
    };

    let bitarray = BitArray::from_bytes(key_hash);
    let hash = hash_leaf_node(key_hash, value_hash);

    compute_and_compare_root_hash(root_hash, &bitarray, sibling_hashes, hash)
}

pub fn verify_non_membership(
    root_hash: &Hash,
    key_hash:  &Hash,
    proof:     &Proof,
) -> Result<(), ProofError> {
    let Proof::NonMembership { node, sibling_hashes } = proof else {
        return Err(ProofError::IncorrectProofType {
            expect: "non-membership",
            actual: "membership",
        });
    };

    let bitarray = BitArray::from_bytes(key_hash);
    let hash = match node {
        // if the node given is an internal node, we check the bit at the depth.
        // if the bit is a 0, it must not have a left child; if the bit is a 1,
        // it must not have a right child.
        Node::Internal(InternalNode { left_child, right_child }) => {
            match (bitarray.bit_at_index(sibling_hashes.len()), left_child, right_child) {
                (0, Some(_), _) | (1, _, Some(_)) => {
                    return Err(ProofError::UnexpectedChild);
                },
                _ => hash_internal_node(hash_of(left_child), hash_of(right_child)),
            }
        },
        // if the node given is a leaf, it's bit path must share a common prefix
        // with the key we want to prove not exist.
        Node::Leaf(LeafNode { key_hash, value_hash }) => {
            let non_exist_bitarray = BitArray::from_bytes(key_hash);
            let exist_bits = bitarray.reverse_iterate_from_index(sibling_hashes.len());
            let non_exist_bits = non_exist_bitarray.reverse_iterate_from_index(sibling_hashes.len());
            if exist_bits.zip(non_exist_bits).any(|(a, b)| a != b) {
                return Err(ProofError::NotCommonPrefix);
            }
            hash_leaf_node(key_hash, value_hash)
        },
    };

    compute_and_compare_root_hash(root_hash, &bitarray, sibling_hashes, hash)
}

fn compute_and_compare_root_hash(
    root_hash: &Hash,
    bitarray: &BitArray,
    sibling_hashes: &[Option<Hash>],
    mut hash: Hash,
) -> Result<(), ProofError> {
    for (bit, sibling_hash) in bitarray.reverse_iterate_from_index(sibling_hashes.len()).zip(sibling_hashes) {
        if bit == 0 {
            hash = hash_internal_node(Some(&hash), sibling_hash.as_ref());
        } else {
            hash = hash_internal_node(sibling_hash.as_ref(), Some(&hash));
        }
    }

    if hash != *root_hash {
        return Err(ProofError::RootHashMismatch {
            computed: hash,
            actual: root_hash.clone(),
        });
    }

    Ok(())
}

// ----------------------------------- tests -----------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proving_membership() {
        todo!()
    }

    #[test]
    fn proving_non_membership() {
        todo!()
    }
}
