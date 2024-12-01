//! This module contains the contract entrypoints for [`TendermintContext`].

use anyhow::Result;
use dango_types::ibc_client::{ExecuteMsg, InstantiateMsg, QueryMsg};
use grug::Json;
//use grug::Binary;
//use ibc_client_tendermint::types::proto::v1::{
//    ClientState as RawTmClientState, ConsensusState as RawTmConsensusState,
//};
use ibc_client_tendermint::{
    client_state::ClientState as ClientStateWrapper,
    //consensus_state::ConsensusState as ConsensusStateWrapper,
};
use ibc_core_client::context::prelude::ClientStateExecution;
use ibc_core_client::context::ClientValidationContext;
use ibc_primitives::proto::{Any, Protobuf};
use prost::Message;

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
        let _client_state = self.client_state(&client_id)?;

        match msg {
            ExecuteMsg::UpdateClient(_) => todo!(),
            ExecuteMsg::Misbehaviour(_) => todo!(),
            ExecuteMsg::UpgradeClient(_) => todo!(),
        }
    }

    /// Queries with the given [`QueryMsg`] message.
    /// # Errors
    /// Returns an error if the underlying light client encounters an error.
    #[allow(clippy::needless_pass_by_value)]
    pub fn query(&self, msg: QueryMsg) -> Result<Json> {
        let client_id = self.client_id();
        let _client_state = self.client_state(&client_id)?;

        match msg {
            QueryMsg::Status(_) => todo!(),
            QueryMsg::TimestampAtHeight(_) => todo!(),
            QueryMsg::VerifyMembership(_) => todo!(),
            QueryMsg::VerifyNonMembership(_) => todo!(),
        }
    }
}
