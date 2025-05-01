use {
    dango_types::lending::Market,
    grug::{Addr, Denom, Map, Uint128},
    std::collections::BTreeMap,
};

/// The markets that are available to borrow from.
///
/// ```raw
/// denom => market
/// ```
pub const MARKETS: Map<&Denom, Market> = Map::new("market");

/// The deposits of all lenders.
///
/// ```raw
/// lender_addr => (denom => amount_scaled)
/// ```
pub const ASSETS: Map<Addr, BTreeMap<Denom, Uint128>> = Map::new("asset");

/// The debts of all borrowers.
///
/// ```raw
/// borrower_addr => (denom => amount_scaled)
/// ```
pub const DEBTS: Map<Addr, BTreeMap<Denom, Uint128>> = Map::new("debt");
