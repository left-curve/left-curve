use {
    crate::ibc::host::ClientId,
    grug::{Binary, Json},
};

pub type Height = u64;

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Verify the creation of a client instance.
    ///
    /// Return the latest consensus height, and client and consensus encoded as
    /// raw bytes using the appropriate encoding scheme according to the client's
    /// specification (e.g. Protobuf for 07-tendermint).
    #[returns(VerifyCreationResponse)]
    VerifyCreation {
        client_state: Json,
        consensus_state: Json,
    },
    /// Verify the client message and perform state transition.
    ///
    /// This can either be advance the consensus height given a block header, or
    /// freeze the client given evidence of a misbehavior.
    #[returns(VerifyClientMessageResponse)]
    VerifyClientMessage {
        client_id: ClientId,
        client_message: Json,
    },
}

#[grug::derive(Serde)]
pub struct VerifyCreationResponse {
    pub latest_height: Height,
    pub raw_client_state: Binary,
    pub raw_consensus_state: Binary,
}

#[grug::derive(Serde)]
pub struct VerifyClientMessageResponse {
    pub height: Height,
    pub raw_client_state: Binary,
    pub raw_consensus_state: Binary,
}
