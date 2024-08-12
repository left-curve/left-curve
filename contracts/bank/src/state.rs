use {
    grug_storage::Map,
    grug_types::{Addr, Uint256},
};

/// Total supplies of tokens, indexed by denoms.
pub const SUPPLIES: Map<&str, Uint256> = Map::new("s");

/// Token balances, indexed first by user addresses, then by denoms.
pub const BALANCES_BY_ADDR: Map<(Addr, &str), Uint256> = Map::new("bu");

/// Token balances, indexed first by denoms, then by user addresses.
pub const BALANCES_BY_DENOM: Map<(&str, Addr), Uint256> = Map::new("bd");
