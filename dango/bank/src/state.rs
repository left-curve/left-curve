use {
    dango_types::bank::Metadata,
    grug::{Addr, Coins, Denom, Map, Part, Uint128},
};

pub const NAMESPACE_OWNERS: Map<&Part, Addr> = Map::new("namespace_owner");

pub const METADATAS: Map<&Denom, Metadata> = Map::new("metadata");

pub const SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

pub const BALANCES: Map<(&Addr, &Denom), Uint128> = Map::new("balance");

// (recipient, sender) -> coins
pub const NON_EXISTING_DEPOSITS: Map<(&Addr, &Addr), Coins> = Map::new("non_existing_deposit");
