use grug::{Addr, Coin, HexByteArray, Item, Map, UniqueVec};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

pub const STORAGE_LOCATIONS: Map<HexByteArray<20>, UniqueVec<String>> =
    Map::new("storage_location");

pub const ANNOUNCE_FEE_PER_BYTE: Item<Coin> = Item::new("announce_fee_per_byte");
