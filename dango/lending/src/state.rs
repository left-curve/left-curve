use {
    dango_types::lending::Market,
    grug::{Addr, Coins, Denom, Map},
};

/// The markets that are available to borrow from. The key is the denom of the
/// borrowable asset.
pub const MARKETS: Map<&Denom, Market> = Map::new("market");

/// The debts of all margin accounts. The key is the address of the margin
/// account.
pub const DEBTS: Map<Addr, Coins> = Map::new("debt");
