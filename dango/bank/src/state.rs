use {
    dango_types::bank::Metadata,
    grug::{Addr, Coins, Denom, Map, Part, Uint128},
};

pub const NAMESPACE_OWNERS: Map<&Part, Addr> = Map::new("namespace_owner");

pub const METADATAS: Map<&Denom, Metadata> = Map::new("metadata");

pub const SUPPLIES: Map<&Denom, Uint128> = Map::new("supply");

pub const BALANCES: Map<(&Addr, &Denom), Uint128> = Map::new("balance");

// (sender, recipient) -> coins
pub const ORPHANED_TRANSFERS: Map<(Addr, Addr), Coins> = Map::new("orphaned_deposit");
