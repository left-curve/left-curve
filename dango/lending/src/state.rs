use {
    dango_types::lending::Market,
    grug::{Addr, Coins, Denom, Map},
};

pub const MARKETS: Map<&Denom, Market> = Map::new("market");

/// The coins that each margin account has borrowed from the lending pool.
pub const LIABILITIES: Map<Addr, Coins> = Map::new("debts");
