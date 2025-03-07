use {
    grug::{Addr, HexByteArray, Item, Map, UniqueVec},
    hyperlane_types::va::Config,
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

pub const STORAGE_LOCATIONS: Map<HexByteArray<20>, UniqueVec<String>> =
    Map::new("storage_location");

pub const CONFIG: Item<Config> = Item::new("config");
