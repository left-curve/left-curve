use {
    dango_types::ibc::{
        client::Height,
        host::{Client, ClientId, ClientType, Commitment},
    },
    grug::{Addr, Counter, Json, Map, Raw},
};

pub const CLIENT_REGISTRY: Map<ClientType, Addr> = Map::new("client_registry");

pub const NEXT_CLIENT_ID: Counter<ClientId> = Counter::new("next_client_id", 0, 1);

pub const CLIENTS: Map<ClientId, Client> = Map::new("client");

pub const CLIENT_STATES: Map<ClientId, Json> = Map::new("client_state");

pub const CONSENSUS_STATES: Map<(ClientId, Height), Json> = Map::new("consensus_state");

pub const COMMITMENTS: Map<&Commitment, Commitment, Raw> = Map::new("commitment");
