use {
    grug::{Addr, Hash256, HexByteArray, Item, Map, Set},
    hyperlane_types::mailbox::Domain,
    std::collections::BTreeSet,
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

pub const LOCAL_DOMAIN: Item<Domain> = Item::new("local_domain");

pub const VALIDATORS: Set<HexByteArray<20>> = Set::new("validators");

pub const STORAGE_LOCATIONS: Map<HexByteArray<20>, BTreeSet<String>> =
    Map::new("storage_locations");

pub const REPLAY_PROTECTIONS: Set<Hash256> = Set::new("replay_protections");
