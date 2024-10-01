use grug::{Addr, Denom, Map, Part, Uint256};

pub const SUPPLIES: Map<&Denom, Uint256> = Map::new("supply");

pub const BALANCES: Map<(&Addr, &Denom), Uint256> = Map::new("balance");

pub const NAMESPACE_OWNERS: Map<&Part, Addr> = Map::new("namespace_owner");
