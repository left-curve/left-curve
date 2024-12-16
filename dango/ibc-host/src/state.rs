use {
    dango_types::ibc::host::ClientType,
    grug::{Addr, Map},
};

pub const CLIENT_IMPLS: Map<ClientType, Addr> = Map::new("client_impl");
