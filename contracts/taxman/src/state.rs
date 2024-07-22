use {crate::Config, grug_storage::Item};

pub const CONFIG: Item<Config> = Item::new("fee_denom");
