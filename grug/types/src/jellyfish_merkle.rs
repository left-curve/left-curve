use {
    crate::Hash256,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Proof {
    Membership(MembershipProof),
    NonMembership(NonMembershipProof),
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MembershipProof {
    pub sibling_hashes: Vec<Option<Hash256>>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct NonMembershipProof {
    pub node: ProofNode,
    pub sibling_hashes: Vec<Option<Hash256>>,
}

/// `ProofNode` is just like `Node`, but for internal nodes it omits the child
/// versions, which aren't needed for proving, only including child node hashes.
/// This reduces proof sizes.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum ProofNode {
    Internal {
        left_hash: Option<Hash256>,
        right_hash: Option<Hash256>,
    },
    Leaf {
        key_hash: Hash256,
        value_hash: Hash256,
    },
}
