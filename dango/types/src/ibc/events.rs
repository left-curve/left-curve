use crate::ibc::{
    client::Height,
    host::{ClientId, ClientType},
};

#[grug::derive(Serde)]
pub struct ClientCreated {
    pub client_id: ClientId,
    pub client_type: ClientType,
    pub consensus_height: Height,
}

#[grug::derive(Serde)]
pub struct ClientUpdated {
    pub client_id: ClientId,
    pub client_type: ClientType,
    pub height: Height,
}
