use grug::{Addr, Coin, Denom, Item, Map};

pub const DENOM_CREATION_FEE: Item<Coin> = Item::new("denom_creation_fee");

pub const DENOM_ADMINS: Map<&Denom, Addr> = Map::new("denom");
