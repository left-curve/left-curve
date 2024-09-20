use {
    grug_math::Uint256,
    grug_storage::Map,
    grug_types::{Addr, Denom},
};

/// Total supplies of tokens, indexed by denoms.
pub const SUPPLIES: Map<&Denom, Uint256> = Map::new("s");

/// Token balances, indexed first by user addresses, then by denoms.
pub const BALANCES_BY_ADDR: Map<(Addr, &Denom), Uint256> = Map::new("bu");

/// Token balances, indexed first by denoms, then by user addresses.
pub const BALANCES_BY_DENOM: Map<(&Denom, Addr), Uint256> = Map::new("bd");
