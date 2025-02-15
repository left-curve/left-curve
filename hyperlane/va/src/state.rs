use grug::{Addr, HexByteArray, Item, Map};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

pub const STORAGE_LOCATIONS: Map<HexByteArray<20>, Vec<String>> = Map::new("storage_location");
