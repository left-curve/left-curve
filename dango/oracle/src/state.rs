use {
    dango_types::oracle::{Config, Price},
    grug::{Addr, Denom, Item, Map, Set, Udec128},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const GUARDIANS: Set<Addr> = Set::new("guardian");

pub const PRICES: Map<&Denom, Price> = Map::new("price");

pub const PRICE_FEEDS: Map<(&Denom, Addr), Udec128> = Map::new("price_feed");
