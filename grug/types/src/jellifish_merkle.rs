use {
    crate::Hash256,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
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
    RootHashMismatch { computed: Hash256, actual: Hash256 },

    // TODO: add more details to the error message?
    #[error("expecting child to not exist but it exists")]
    UnexpectedChild,

    // TODO: add more details to the error message?
    #[error("expecting bitarrays to share a common prefix but they do not")]
    NotCommonPrefix,
}

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
