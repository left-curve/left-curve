use {
    crate::Binary,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IbcClientStatus {
    /// Under the `Active` state, the client can be updated, and can perform
    /// proof verifications.
    Active,
    /// A client is frozen if it has been presented a valid proof of misbehavior.
    /// In this case, the client state is deemed not trustworthy. It cannot be
    /// Further updated, and all membership or non-membership verifications fail.
    /// Social coordination is required to determine what to do with the client.
    Frozen,
    /// A client is expired if it has not been updated for an extended amount of
    /// time. It cannot be updated, but can still perform verifications.
    Expired,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IbcClientUpdateMsg {
    /// Present the client with a new header. The client will verify it and
    /// perform updates to the client and consensus states.
    Update {
        header: Binary,
    },
    /// Present the client with a proof of misbehavior. The client will verify
    /// it and freeze itself.
    UpdateOnMisbehavior {
        misbehavior: Binary,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IbcClientVerifyMsg {
    /// Verify a Merkle memership proof of the given key and value.
    /// Returns nothing if verification succeeds; returns error otherwise.
    VerifyMembership {
        height: u64,
        delay_time_period: u64,
        delay_block_period: u64,
        key: Binary,
        value: Binary,
        proof: Binary,
    },
    /// Verify a Merkle non-membership proof of the given key.
    /// Returns nothing if verification succeeds; returns error otherwise.
    VerifyNonMembership {
        height: u64,
        delay_time_period: u64,
        delay_block_period: u64,
        key: Binary,
        proof: Binary,
    },
}
