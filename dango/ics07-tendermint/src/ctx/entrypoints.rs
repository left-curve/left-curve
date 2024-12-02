//! This module contains the contract entrypoints for [`TendermintContext`].

use {
    anyhow::Result,
    dango_types::ibc_client::{
        ExecuteMsg, InstantiateMsg, MembershipResponse, QueryMsg, StatusResponse,
        TimestampAtHeightResponse,
    },
    grug::{Json, JsonSerExt},
};
// use grug::Binary;
// use ibc_client_tendermint::types::proto::v1::{
//    ClientState as RawTmClientState, ConsensusState as RawTmConsensusState,
//};
use ibc_client_tendermint::{
    client_state::ClientState as ClientStateWrapper,
    // consensus_state::ConsensusState as ConsensusStateWrapper,
};
use {
    ibc_core_client::context::{
        prelude::{ClientStateCommon, ClientStateExecution, ClientStateValidation, ConsensusState},
        ClientValidationContext,
    },
    ibc_core_commitment_types::commitment::{CommitmentPrefix, CommitmentProofBytes},
    ibc_core_host_types::path::{ClientConsensusStatePath, PathBytes},
    ibc_primitives::proto::{Any, Protobuf},
    prost::Message,
};

use super::TendermintContext;

impl TendermintContext<'_> {
    /// Instantiates a new client with the given [`InstantiateMsg`] message.
    /// # Errors
    /// Returns an error if the messages cannot be decoded.
    #[allow(clippy::needless_pass_by_value)]
    pub fn instantiate(&mut self, msg: InstantiateMsg) -> Result<()> {
        let client_state =
            <ClientStateWrapper as Protobuf<Any>>::decode_vec(msg.client_state.as_ref())?;

        let any_consensus_state = Any::decode(&mut msg.consensus_state.as_ref())?;

        client_state.initialise(self, &self.client_id(), any_consensus_state)?;

        Ok(())
    }

    /// Executes the given [`ExecuteMsg`] message.
    /// # Errors
    /// Returns an error if the underlying light client encounters an error.
    #[allow(clippy::needless_pass_by_value)]
    pub fn execute(&mut self, msg: ExecuteMsg) -> Result<()> {
        let client_id = self.client_id();
        let client_state = self.client_state(&client_id)?;

        match msg {
            ExecuteMsg::UpdateClient(msg) => {
                let any_msg = Any::decode(&mut msg.client_message.as_ref())?;
                let _ = client_state.update_state(self, &client_id, any_msg)?;
                Ok(())
            },
            ExecuteMsg::Misbehaviour(msg) => {
                let any_msg = Any::decode(&mut msg.client_message.as_ref())?;
                client_state.update_state_on_misbehaviour(self, &client_id, any_msg)?;
                Ok(())
            },
            ExecuteMsg::UpgradeClient(msg) => {
                let upgrade_client_state = Any::decode(&mut msg.upgrade_client_state.as_ref())?;

                let upgrade_consensus_state =
                    Any::decode(&mut msg.upgrade_consensus_state.as_ref())?;

                let proof_upgrade_client_state =
                    CommitmentProofBytes::try_from(msg.proof_upgrade_client_state.to_vec())?;

                let proof_upgrade_consensus_state =
                    CommitmentProofBytes::try_from(msg.proof_upgrade_consensus_state.to_vec())?;

                let client_cons_state_path = ClientConsensusStatePath::new(
                    client_id.clone(),
                    client_state.latest_height().revision_number(),
                    client_state.latest_height().revision_height(),
                );

                let consensus_state = self.consensus_state(&client_cons_state_path)?;

                client_state.verify_upgrade_client(
                    upgrade_client_state.clone(),
                    upgrade_consensus_state.clone(),
                    proof_upgrade_client_state,
                    proof_upgrade_consensus_state,
                    consensus_state.root(),
                )?;

                client_state.update_state_on_upgrade(
                    self,
                    &client_id,
                    upgrade_client_state,
                    upgrade_consensus_state,
                )?;

                Ok(())
            },
        }
    }

    /// Queries with the given [`QueryMsg`] message.
    /// # Errors
    /// Returns an error if the underlying light client encounters an error.
    #[allow(clippy::needless_pass_by_value)]
    pub fn query(&self, msg: QueryMsg) -> Result<Json> {
        let client_id = self.client_id();
        let client_state = self.client_state(&client_id)?;

        match msg {
            QueryMsg::Status(_) => {
                let status = client_state.status(self, &client_id)?;
                Ok(StatusResponse { status }.to_json_value()?)
            },
            QueryMsg::TimestampAtHeight(msg) => {
                let client_cons_state_path = ClientConsensusStatePath::new(
                    client_id,
                    msg.height.revision_number,
                    msg.height.revision_height,
                );

                let consensus_state = self.consensus_state(&client_cons_state_path)?;
                let timestamp = consensus_state
                    .timestamp()
                    .unix_timestamp_nanos()
                    .try_into()?;

                Ok(TimestampAtHeightResponse { timestamp }.to_json_value()?)
            },
            QueryMsg::VerifyMembership(mut msg) => {
                if msg.path.len() < 2 {
                    anyhow::bail!("path length must be greater than 1")
                }

                let proof = CommitmentProofBytes::try_from(msg.proof.to_vec())?;
                let prefix = CommitmentPrefix::from_bytes(msg.path.remove(0));
                let path_bz = PathBytes::flatten(msg.path);

                let client_cons_state_path = ClientConsensusStatePath::new(
                    self.client_id(),
                    msg.height.revision_number,
                    msg.height.revision_height,
                );

                let consensus_state = self.consensus_state(&client_cons_state_path)?;

                client_state.verify_membership_raw(
                    &prefix,
                    &proof,
                    consensus_state.root(),
                    path_bz,
                    msg.value.to_vec(),
                )?;

                let timestamp = consensus_state
                    .timestamp()
                    .unix_timestamp_nanos()
                    .try_into()?;

                Ok(MembershipResponse { timestamp }.to_json_value()?)
            },
            QueryMsg::VerifyNonMembership(mut msg) => {
                if msg.path.len() < 2 {
                    anyhow::bail!("path length must be greater than 1")
                }

                let proof = CommitmentProofBytes::try_from(msg.proof.to_vec())?;
                let prefix = CommitmentPrefix::from_bytes(msg.path.remove(0));
                let path_bz = PathBytes::flatten(msg.path);

                let client_cons_state_path = ClientConsensusStatePath::new(
                    self.client_id(),
                    msg.height.revision_number,
                    msg.height.revision_height,
                );

                let consensus_state = self.consensus_state(&client_cons_state_path)?;

                client_state.verify_non_membership_raw(
                    &prefix,
                    &proof,
                    consensus_state.root(),
                    path_bz,
                )?;

                let timestamp = consensus_state
                    .timestamp()
                    .unix_timestamp_nanos()
                    .try_into()?;

                Ok(MembershipResponse { timestamp }.to_json_value()?)
            },
        }
    }
}
