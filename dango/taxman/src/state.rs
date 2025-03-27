use {
    dango_types::taxman::{Config, FeePayments, FeeType},
    grug::{Addr, Item, Map, Timestamp, Uint128},
    std::collections::BTreeMap,
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const WITHHELD_FEE: Item<(Config, Uint128)> = Item::new("withheld_fee");

/// Map of fees collected by the taxman for a specific user.
///
/// The key is a tuple of (user address, timestamp).
/// The value is a BTreeMap of fee types to FeePayments (which contains the
/// coins and the usd value).
pub const FEES_BY_USER: Map<(Addr, Timestamp), BTreeMap<FeeType, FeePayments>> = Map::new("fees");
