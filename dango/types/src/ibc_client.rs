//! This crate defines the types required by all IBC light clients.

#![deny(clippy::nursery, clippy::pedantic, warnings)]

use grug::Binary;
pub use {execute::*, query::*};

/// Instantiate message for all light client contracts
#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// The initial client state.
    pub client_state: Binary,
    /// The initial consensus state.
    pub consensus_state: Binary,
}

/// Execute message for all light client contracts
#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Update the client state.
    UpdateClient(UpdateClientMsg),
    /// Freeze the client.
    Misbehaviour(MisbehaviourMsg),
    /// Update the client on counterparty chain upgrade.
    UpgradeClient(UpgradeClientMsg),
}

/// Query messages for all light client contracts
#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the client status.
    #[returns(StatusResponse)]
    Status(StatusMsg),
    /// Query the timestamp at a given height.
    #[returns(TimestampAtHeightResponse)]
    TimestampAtHeight(TimestampAtHeightMsg),
    /// Verify membership of a key-value pair in a Merkle tree.
    #[returns(MembershipResponse)]
    VerifyMembership(VerifyMembershipMsg),
    /// Verify non-membership of a key-value pair in a Merkle tree.
    #[returns(MembershipResponse)]
    VerifyNonMembership(VerifyNonMembershipMsg),
}

mod execute {
    use super::Binary;

    /// Update the client state.
    #[grug::derive(Serde)]
    pub struct UpdateClientMsg {
        /// Update message.
        pub client_message: Binary,
    }

    /// Misbehaviour message.
    #[grug::derive(Serde)]
    pub struct MisbehaviourMsg {
        /// Misbehaviour message.
        pub client_message: Binary,
    }

    /// Update client on upgrade.
    #[grug::derive(Serde)]
    pub struct UpgradeClientMsg {
        /// Client state after upgrade.
        pub upgrade_client_state: Binary,
        /// Consensus state after upgrade.
        pub upgrade_consensus_state: Binary,
        /// Proof of upgrade client state.
        pub proof_upgrade_client_state: Binary,
        /// Proof of upgrade consensus state.
        pub proof_upgrade_consensus_state: Binary,
    }
}

mod query {
    pub use responses::*;
    use {super::Binary, ibc_proto::ibc::core::client::v1::Height};

    /// Status Query message.
    #[grug::derive(Serde)]
    pub struct StatusMsg {}

    /// Query the timestamp at a given height.
    #[grug::derive(Serde)]
    pub struct TimestampAtHeightMsg {
        /// The counterparty chain height.
        pub height: Height,
    }

    /// Verify membership of a key-value pair in a Merkle tree.
    #[grug::derive(Serde)]
    pub struct VerifyMembershipMsg {
        /// The proof to verify.
        pub proof: Binary,
        /// The path at which the value is stored.
        pub path: Vec<Binary>,
        /// The value to verify the membership of.
        pub value: Binary,
        /// The height of the proof.
        pub height: Height,
    }

    /// Verify non-membership of a key-value pair in a Merkle tree.
    #[grug::derive(Serde)]
    pub struct VerifyNonMembershipMsg {
        /// The proof to verify.
        pub proof: Binary,
        /// The path to verify non-membership at.
        pub path: Vec<Binary>,
        /// The height of the proof.
        pub height: Height,
    }

    mod responses {
        use ibc_core_client_types::Status;

        /// The response to `QueryMsg::Status`
        #[grug::derive(Serde)]
        pub struct StatusResponse {
            /// The status of the client
            pub status: Status,
        }

        /// The response to `QueryMsg::TimestampAtHeight`
        #[grug::derive(Serde)]
        pub struct TimestampAtHeightResponse {
            /// The timestamp at the given height
            /// in nanoseconds since the Unix epoch.
            pub timestamp: u64,
        }

        /// The response to both membership queries.
        #[grug::derive(Serde)]
        pub struct MembershipResponse {
            /// The timestamp at height of the proof
            /// in nanoseconds since the Unix epoch.
            pub timestamp: u64,
        }
    }
}
