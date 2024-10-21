use {
    dango_types::token_factory::Config,
    grug::{Addr, Denom, Item, Map},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const ADMINS: Map<&Denom, Addr> = Map::new("admin");
