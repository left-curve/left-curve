use {
    grug::{Addr, HexByteArray, Item, Map},
    std::collections::BTreeSet,
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

pub const STORAGE_LOCATIONS: Map<HexByteArray<20>, BTreeSet<String>> = Map::new("storage_location");
