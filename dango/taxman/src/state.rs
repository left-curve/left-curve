use {
    dango_types::taxman::Config,
    grug::{Item, Uint128},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const WITHHELD_FEE: Item<(Config, Uint128)> = Item::new("withheld_fee");
