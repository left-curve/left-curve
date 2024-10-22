use {
    crate::{Binary, Timestamp},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Code {
    pub code: Binary,
    pub status: CodeStatus,
}

#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Copy, Debug, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum CodeStatus {
    /// The code is not used by any contract.
    Orphaned {
        /// The time since which the code has been orphaned.
        since: Timestamp,
    },
    /// The code is used by at least one contract.
    InUse {
        /// The number of contracts that use the code.
        usage: u32,
    },
}
