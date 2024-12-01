//! Implementation of the [`ClientValidationContext`], [`ClientExecutionContext`] and
//! [`ExtClientValidationContext`] traits for the [`TendermintContext`] type.

use grug::Empty;
use ibc_client_tendermint::types::proto::v1::{
    ClientState as RawTmClientState, ConsensusState as RawTmConsensusState,
};
use ibc_client_tendermint::{
    client_state::ClientState as ClientStateWrapper,
    consensus_state::ConsensusState as ConsensusStateWrapper,
};
use ibc_core_client::context::{
    ClientExecutionContext, ClientValidationContext, ExtClientValidationContext,
};
use ibc_core_client::types::Height;
use ibc_core_host_types::error::HostError;
use ibc_core_host_types::identifiers::ClientId;
use ibc_core_host_types::path::{ClientConsensusStatePath, ClientStatePath};
use ibc_primitives::proto::Protobuf;
use ibc_primitives::Timestamp;

use super::HeightTravel;
use super::TendermintContext;
use super::CONSENSUS_STATE_HEIGHT_MAP;

impl ClientValidationContext for TendermintContext<'_> {
    type ClientStateRef = ClientStateWrapper;
    type ConsensusStateRef = ConsensusStateWrapper;

    fn client_state(&self, _client_id: &ClientId) -> Result<Self::ClientStateRef, HostError> {
        let client_state_value = self.retrieve(ClientStatePath::leaf())?;

        let cs_wrapper =
            <ClientStateWrapper as Protobuf<RawTmClientState>>::decode_vec(&client_state_value)
                .map_err(HostError::invalid_state)?;

        Ok(cs_wrapper)
    }

    fn consensus_state(
        &self,
        client_cons_state_path: &ClientConsensusStatePath,
    ) -> Result<Self::ConsensusStateRef, HostError> {
        let consensus_state_value = self.retrieve(client_cons_state_path.leaf())?;

        let cs_wrapper = <ConsensusStateWrapper as Protobuf<RawTmConsensusState>>::decode_vec(
            &consensus_state_value,
        )
        .map_err(HostError::invalid_state)?;

        Ok(cs_wrapper)
    }

    fn client_update_meta(
        &self,
        _client_id: &ClientId,
        height: &Height,
    ) -> Result<(Timestamp, Height), HostError> {
        let time_key = self.client_update_time_key(height);

        let time_vec = self.retrieve(time_key)?;

        let time = u64::from_be_bytes(
            time_vec
                .try_into()
                .map_err(|_| HostError::invalid_state("time key cannot be converted to u64"))?,
        );

        let timestamp = Timestamp::from_nanoseconds(time);

        let height_key = self.client_update_height_key(height);

        let revision_height_vec = self.retrieve(height_key)?;

        let revision_height = u64::from_be_bytes(revision_height_vec.try_into().map_err(|_| {
            HostError::invalid_state("revision height key cannot be converted to u64")
        })?);

        let height = Height::new(0, revision_height).map_err(HostError::invalid_state)?;

        Ok((timestamp, height))
    }
}

impl ClientExecutionContext for TendermintContext<'_> {
    type ClientStateMut = ClientStateWrapper;

    fn store_client_state(
        &mut self,
        _client_state_path: ClientStatePath,
        client_state: Self::ClientStateMut,
    ) -> Result<(), HostError> {
        let prefixed_key = self.prefixed_key(ClientStatePath::leaf());

        let encoded_client_state =
            <ClientStateWrapper as Protobuf<RawTmClientState>>::encode_vec(client_state);

        self.insert(prefixed_key, encoded_client_state);

        Ok(())
    }

    fn store_consensus_state(
        &mut self,
        consensus_state_path: ClientConsensusStatePath,
        consensus_state: Self::ConsensusStateRef,
    ) -> Result<(), HostError> {
        let prefixed_key = self.prefixed_key(consensus_state_path.leaf());

        let encoded_consensus_state =
            <ConsensusStateWrapper as Protobuf<RawTmConsensusState>>::encode_vec(consensus_state);

        self.insert(prefixed_key, encoded_consensus_state);

        Ok(())
    }

    fn delete_consensus_state(
        &mut self,
        consensus_state_path: ClientConsensusStatePath,
    ) -> Result<(), HostError> {
        let prefixed_key = self.prefixed_key(consensus_state_path.leaf());

        self.remove(prefixed_key);

        Ok(())
    }

    fn store_update_meta(
        &mut self,
        _client_id: ClientId,
        height: Height,
        host_timestamp: Timestamp,
        host_height: Height,
    ) -> Result<(), HostError> {
        let time_key = self.client_update_time_key(&height);

        let prefixed_time_key = self.prefixed_key(time_key);

        let time_vec = host_timestamp.nanoseconds().to_be_bytes();

        self.insert(prefixed_time_key, time_vec);

        let height_key = self.client_update_height_key(&height);

        let prefixed_height_key = self.prefixed_key(height_key);

        let revision_height_vec = host_height.revision_height().to_be_bytes();

        self.insert(prefixed_height_key, revision_height_vec);

        CONSENSUS_STATE_HEIGHT_MAP
            .save(
                self.storage_mut(),
                (height.revision_number(), height.revision_height()),
                &Empty {},
            )
            .map_err(HostError::failed_to_store)?;

        Ok(())
    }

    fn delete_update_meta(
        &mut self,
        _client_id: ClientId,
        height: Height,
    ) -> Result<(), HostError> {
        let time_key = self.client_update_time_key(&height);

        let prefixed_time_key = self.prefixed_key(time_key);

        self.remove(prefixed_time_key);

        let height_key = self.client_update_height_key(&height);

        let prefixed_height_key = self.prefixed_key(height_key);

        self.remove(prefixed_height_key);

        CONSENSUS_STATE_HEIGHT_MAP.remove(
            self.storage_mut(),
            (height.revision_number(), height.revision_height()),
        );

        Ok(())
    }
}

impl ExtClientValidationContext for TendermintContext<'_> {
    fn host_timestamp(&self) -> Result<Timestamp, HostError> {
        let time = self.block().timestamp;

        let host_timestamp = Timestamp::from_nanoseconds(time.into_nanos().try_into().unwrap());

        Ok(host_timestamp)
    }

    fn host_height(&self) -> Result<Height, HostError> {
        let host_height = Height::new(0, self.block().height).map_err(HostError::invalid_state)?;

        Ok(host_height)
    }

    fn consensus_state_heights(&self, _client_id: &ClientId) -> Result<Vec<Height>, HostError> {
        let heights = self.get_heights()?;

        Ok(heights)
    }
    fn next_consensus_state(
        &self,
        client_id: &ClientId,
        height: &Height,
    ) -> Result<Option<Self::ConsensusStateRef>, HostError> {
        self.get_adjacent_height(height, HeightTravel::Next)?
            .map_or(Ok(None), |h| {
                let cons_state_path = ClientConsensusStatePath::new(
                    client_id.clone(),
                    h.revision_number(),
                    h.revision_height(),
                );
                self.consensus_state(&cons_state_path).map(Some)
            })
    }

    fn prev_consensus_state(
        &self,
        client_id: &ClientId,
        height: &Height,
    ) -> Result<Option<Self::ConsensusStateRef>, HostError> {
        self.get_adjacent_height(height, HeightTravel::Prev)?
            .map_or(Ok(None), |h| {
                let cons_state_path = ClientConsensusStatePath::new(
                    client_id.clone(),
                    h.revision_number(),
                    h.revision_height(),
                );
                self.consensus_state(&cons_state_path).map(Some)
            })
    }
}
