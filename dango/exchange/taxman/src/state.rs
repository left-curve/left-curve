use {dango_math::Uint128, dango_storage::Item, dango_types::taxman::Config};

pub const CONFIG: Item<Config> = Item::new("config");

pub const WITHHELD_FEE: Item<(Config, Uint128)> = Item::new("withheld_fee");
