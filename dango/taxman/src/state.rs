use {
    dango_types::taxman::Config,
    grug::{Coin, Item},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const WITHHELD_COIN: Item<Coin> = Item::new("withheld");
