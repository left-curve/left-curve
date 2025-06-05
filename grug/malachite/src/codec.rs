use {
    crate::context::Context,
    grug::{BorshDeExt, BorshSerExt, StdError},
    malachitebft_app::{
        consensus::LivenessMsg,
        streaming::StreamMessage,
        types::{ProposedValue, SignedConsensusMsg, codec::Codec},
    },
    malachitebft_sync::{Request, Response, Status},
};

use crate::ctx;

pub struct Borsh;

impl Codec<ctx!(ProposalPart)> for Borsh {
    type Error = StdError;

    fn decode(
        &self,
        bytes: malachitebft_network::Bytes,
    ) -> Result<ctx!(ProposalPart), Self::Error> {
        bytes.deserialize_borsh()
    }

    fn encode(&self, msg: &ctx!(ProposalPart)) -> Result<malachitebft_network::Bytes, Self::Error> {
        msg.to_borsh_vec().map(Into::into)
    }
}

impl Codec<SignedConsensusMsg<Context>> for Borsh {
    type Error = StdError;

    fn decode(
        &self,
        bytes: malachitebft_network::Bytes,
    ) -> Result<SignedConsensusMsg<Context>, Self::Error> {
        bytes.deserialize_borsh()
    }

    fn encode(
        &self,
        msg: &SignedConsensusMsg<Context>,
    ) -> Result<malachitebft_network::Bytes, Self::Error> {
        msg.to_borsh_vec().map(Into::into)
    }
}

impl Codec<LivenessMsg<Context>> for Borsh {
    type Error = StdError;

    fn decode(
        &self,
        bytes: malachitebft_network::Bytes,
    ) -> Result<LivenessMsg<Context>, Self::Error> {
        bytes.deserialize_borsh()
    }

    fn encode(
        &self,
        msg: &LivenessMsg<Context>,
    ) -> Result<malachitebft_network::Bytes, Self::Error> {
        msg.to_borsh_vec().map(Into::into)
    }
}

impl Codec<StreamMessage<ctx!(ProposalPart)>> for Borsh {
    type Error = StdError;

    fn decode(
        &self,
        bytes: malachitebft_network::Bytes,
    ) -> Result<StreamMessage<ctx!(ProposalPart)>, Self::Error> {
        bytes.deserialize_borsh()
    }

    fn encode(
        &self,
        msg: &StreamMessage<ctx!(ProposalPart)>,
    ) -> Result<malachitebft_network::Bytes, Self::Error> {
        msg.to_borsh_vec().map(Into::into)
    }
}

impl Codec<Status<Context>> for Borsh {
    type Error = StdError;

    fn decode(&self, bytes: malachitebft_network::Bytes) -> Result<Status<Context>, Self::Error> {
        bytes.deserialize_borsh()
    }

    fn encode(&self, msg: &Status<Context>) -> Result<malachitebft_network::Bytes, Self::Error> {
        msg.to_borsh_vec().map(Into::into)
    }
}

impl Codec<Request<Context>> for Borsh {
    type Error = StdError;

    fn decode(&self, bytes: malachitebft_network::Bytes) -> Result<Request<Context>, Self::Error> {
        bytes.deserialize_borsh()
    }

    fn encode(&self, msg: &Request<Context>) -> Result<malachitebft_network::Bytes, Self::Error> {
        msg.to_borsh_vec().map(Into::into)
    }
}

impl Codec<Response<Context>> for Borsh {
    type Error = StdError;

    fn decode(&self, bytes: malachitebft_network::Bytes) -> Result<Response<Context>, Self::Error> {
        bytes.deserialize_borsh()
    }

    fn encode(&self, msg: &Response<Context>) -> Result<malachitebft_network::Bytes, Self::Error> {
        msg.to_borsh_vec().map(Into::into)
    }
}

impl Codec<ProposedValue<Context>> for Borsh {
    type Error = StdError;

    fn decode(
        &self,
        bytes: malachitebft_network::Bytes,
    ) -> Result<ProposedValue<Context>, Self::Error> {
        bytes.deserialize_borsh()
    }

    fn encode(
        &self,
        msg: &ProposedValue<Context>,
    ) -> Result<malachitebft_network::Bytes, Self::Error> {
        msg.to_borsh_vec().map(Into::into)
    }
}
