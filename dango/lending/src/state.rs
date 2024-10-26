use {
    grug::{Addr, Coins, Denom, Map},
    grug_storage::Set,
};

/// The set of whitelisted denoms that can be borrowed.
pub const WHITELISTED_DENOMS: Set<Denom> = Set::new("whitelisted_denoms");

/// The coins that each margin account has borrowed from the lending pool.
pub const LIABILITIES: Map<Addr, Coins> = Map::new("debts");
