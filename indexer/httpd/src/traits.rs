use grug_types::BroadcastClient;

pub trait ConsensusClient: BroadcastClient<Error = anyhow::Error> {}

impl<T> ConsensusClient for T where T: BroadcastClient<Error = anyhow::Error> {}
