//! This crate defines the types required by all IBC light clients.

#![deny(clippy::nursery, clippy::pedantic, warnings, missing_docs)]

use grug::Binary;
use ibc_proto::ibc::core::client::v1::Height;

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
#[grug::derive(Serde)]
pub enum QueryMsg {
    /// Query the client status.
    Status(StatusMsg),
    /// Query the timestamp at a given height.
    TimestampAtHeight(TimestampAtHeightMsg),
}

/// Status Query message.
#[grug::derive(Serde)]
pub struct StatusMsg {}

/// Query the timestamp at a given height.
#[grug::derive(Serde)]
pub struct TimestampAtHeightMsg {
    /// The counterparty chain height.
    pub height: Height,
}

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
