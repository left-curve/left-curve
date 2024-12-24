use {
    grug::{Counter, Hash256, Item, Set},
    hyperlane_types::mailbox::Config,
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const NONCE: Counter<u32> = Counter::new("nonce", 0, 1);

pub const DELIVERIES: Set<Hash256> = Set::new("delivery");
