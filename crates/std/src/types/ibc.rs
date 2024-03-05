use {
    crate::Binary,
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
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
pub enum IbcClientExecuteMsg {
    /// Present the client with a new header. The client will verify it and
    /// perform updates to the client and consensus states.
    Update {
        /// The `header` is given as an opaque bytes. It is up to the client
        /// implementation to interpret it and perform the client state update.
        header: Binary,
    },
    /// Present the client with a proof of misbehavior. The client will verify
    /// it and freeze itself.
    UpdateOnMisbehavior {
        /// Similar to the `header`, the `misbehavior` is opaque. It is up to
        /// the client implementation to interpret it.
        misbehavior: Binary,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IbcClientQueryMsg {
    /// Query the client and consensus states.
    /// Returns: IbcClientStateResponse
    State {},
    /// Verify a Merkle memership proof of the given path and value.
    /// Returns: ()
    VerifyMembership {
        height: u64,
        delay_time_period: u64,
        delay_block_period: u64,
        path: Binary,
        data: Binary,
        /// Similar to `header` and `misbehavior`, the `proof` is opaque. It is
        /// up to the client implementation to interpret it.
        proof: Binary,
    },
    /// Verify a Merkle non-membership proof of the given path.
    /// Returns: ()
    VerifyNonMembership {
        height: u64,
        delay_time_period: u64,
        delay_block_period: u64,
        path: Binary,
        proof: Binary,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IbcClientQueryResponse {
    State(IbcClientStateResponse),
    // the verify methods do not have return values. if the query succeeds then
    // it means the proof is verified; otherwise, an error is thrown.
    VerifyMembership,
    VerifyNonMembership,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IbcClientStateResponse {
    pub client_state: Binary,
    pub consensus_state: Binary,
}
