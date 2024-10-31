use {
    dango_types::lending::{CollateralPower, Market},
    grug::{Addr, Coins, Denom, Item, Map},
    std::collections::BTreeMap,
};

/// The markets that are available to borrow from. The key is the denom of the borrowable asset.
pub const MARKETS: Map<&Denom, Market> = Map::new("market");

/// The powers of all collateral tokens. This is the adjustment factor for the collateral value of
/// a given collateral token. Meaning, if the collateral token has a power of 0.9, then the value of
/// the collateral token is 90% of its actual value.
pub const COLLATERAL_POWERS: Item<BTreeMap<Denom, CollateralPower>> = Item::new("collateral_power");

/// The debts of all margin accounts. The key is the address of the margin account.
pub const DEBTS: Map<Addr, Coins> = Map::new("debt");
