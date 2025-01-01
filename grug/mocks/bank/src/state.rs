use {
    grug_math::Uint128,
    grug_storage::Map,
    grug_types::{Addr, Denom},
};

/// Total supplies of tokens, indexed by denoms.
pub const SUPPLIES: Map<&Denom, Uint128> = Map::new("s");

/// Token balances, indexed first by user addresses, then by denoms.
pub const BALANCES_BY_ADDR: Map<(Addr, &Denom), Uint128> = Map::new("bu");

/// Token balances, indexed first by denoms, then by user addresses.
pub const BALANCES_BY_DENOM: Map<(&Denom, Addr), Uint128> = Map::new("bd");
