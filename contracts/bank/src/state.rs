use grug::{Addr, Map, Uint128};

/// Total supplies of tokens, indexed by denoms.
pub const SUPPLIES: Map<&str, Uint128> = Map::new("s");

/// Token balances, indexed first by user addresses, then by denoms.
pub const BALANCES_BY_ADDR: Map<(&Addr, &str), Uint128> = Map::new("bu");

/// Token balances, indexed first by denoms, then by user addresses.
pub const BALANCES_BY_DENOM: Map<(&str, &Addr), Uint128> = Map::new("bd");
