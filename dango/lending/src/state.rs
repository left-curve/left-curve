use {
    dango_types::lending::Market,
    grug::{Addr, Coins, Denom, Map},
};

pub const MARKETS: Map<&Denom, Market> = Map::new("market");

pub const DEBTS: Map<Addr, Coins> = Map::new("debt");
