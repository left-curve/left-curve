use {
    crate::{Binary, Json},
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

/// The possible statuses that an IBC client can be in.
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

/// The query message that the host provides the IBC client contract during the
/// `ibc_client_query` function call.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IbcClientQuery {
    /// Query the client's status,
    Status {},
    /// Verify a Merkle memership proof of the given key and value.
    ///
    /// Returns `Ok(true)` if verification succeeds; `Ok(false)` if fails; `Err`
    /// if an error happened during the verification process.
    VerifyMembership {
        height: u64,
        delay_time_period: u64,
        delay_block_period: u64,
        key: Binary,
        value: Binary,
        proof: Json,
    },
    /// Verify a Merkle non-membership proof of the given key.
    ///
    /// Returns `Ok(true)` if verification succeeds; `Ok(false)` if fails; `Err`
    /// if an error happened during the verification process.
    VerifyNonMembership {
        height: u64,
        delay_time_period: u64,
        delay_block_period: u64,
        key: Binary,
        proof: Json,
    },
}

/// The query response that the IBC client contract must return during the
/// `ibc_client_query` function call.
///
/// Similar to the bank contract, the response _must_ match the query (see the
/// docs on [`BankQueryResponse`] for details.)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IbcClientQueryResponse {
    Status(IbcClientStatus),
    VerifyMembership(bool),
    VerifyNonMembership(bool),
}

impl IbcClientQueryResponse {
    pub fn as_status(self) -> IbcClientStatus {
        let IbcClientQueryResponse::Status(status) = self else {
            panic!("IbcClientQueryResponse is not Status");
        };
        status
    }

    pub fn as_verify_membership(self) -> bool {
        let IbcClientQueryResponse::VerifyMembership(success) = self else {
            panic!("IbcClientQueryResponse is not VerifyMembership");
        };
        success
    }

    pub fn as_verify_non_membership(self) -> bool {
        let IbcClientQueryResponse::VerifyNonMembership(success) = self else {
            panic!("IbcClientQueryResponse is not VerifyNonMembership");
        };
        success
    }
}
