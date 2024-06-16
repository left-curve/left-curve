use grug::{Addr, Map, Uint128};

/// Token balances, indexed by user addresses then denoms.
// TODO: make this an IndexMap that's also indexed by the denom, so that we can
// have a query that lists all holders of a denom.
pub const BALANCES: Map<(&Addr, &str), Uint128> = Map::new("b");

/// Total supplies of tokens, indexed by denoms.
pub const SUPPLIES: Map<&str, Uint128> = Map::new("s");
