use {
    dango_types::gateway::Remote,
    grug::{Addr, Denom, Map, Uint128},
};

pub const ROUTES: Map<(Addr, Remote), Denom> = Map::new("route");

pub const REVERSE_ROUTES: Map<(&Denom, Remote), Addr> = Map::new("reverse_route");

pub const RESERVES: Map<(Addr, Remote), Uint128> = Map::new("reserve");
