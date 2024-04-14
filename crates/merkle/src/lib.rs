mod bitarray;
mod node;
mod proof;
mod tree;

pub use crate::{
    bitarray::{BitArray, BitIterator},
    node::{hash_internal_node, hash_leaf_node, Child, InternalNode, LeafNode, Node},
    proof::{
        verify_membership_proof, verify_non_membership_proof, verify_proof, MembershipProof,
        NonMembershipProof, Proof, ProofError, ProofNode,
    },
    tree::{MerkleTree, DEFAULT_NODE_NAMESPACE, DEFAULT_ORPHAN_NAMESPACE},
};
