use {
    dango_types::bank::Metadata,
    grug::{Addr, Denom, Map, Part, Uint128},
};

pub const SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

pub const BALANCES: Map<(&Addr, &Denom), Uint128> = Map::new("balance");

pub const NAMESPACE_OWNERS: Map<&Part, Addr> = Map::new("namespace_owner");

pub const DENOM_METADATAS: Map<&Denom, Metadata> = Map::new("denom_metadatas");
