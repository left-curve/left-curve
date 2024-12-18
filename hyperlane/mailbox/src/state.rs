use {
    grug::{Counter, Item, Set},
    hyperlane_types::mailbox::{Config, MessageId},
};

pub const CONFIG: Item<Config> = Item::new("config");

pub const NONCE: Counter<u32> = Counter::new("nonce", 0, 1);

pub const LATEST_DISPATCHED_ID: Item<MessageId> = Item::new("latest_dispatched_id");

pub const DELIVERIES: Set<MessageId> = Set::new("delivery");
