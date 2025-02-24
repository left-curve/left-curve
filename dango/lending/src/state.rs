use {
    dango_types::lending::Market,
    grug::{Addr, Denom, Map, Udec256},
    std::collections::BTreeMap,
};

/// The markets that are available to borrow from. The key is the denom of the
/// borrowable asset.
pub const MARKETS: Map<&Denom, Market> = Map::new("market");

/// The debts of all margin accounts. The key is a the address of the
/// margin account. The value is a BTreeMap of the denom of the debt and the
/// amount of debt scaled by the borrow index.
pub const DEBTS: Map<Addr, BTreeMap<Denom, Udec256>> = Map::new("debt");
