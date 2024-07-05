use {
    crate::PublicKey,
    grug_storage::{Counter, Item},
};

/// The account's public key.
pub const PUBLIC_KEY: Item<PublicKey> = Item::new("pk");

/// The account's sequence number, also known as "nonce" in Ethereum.
pub const SEQUENCE: Counter<u32> = Counter::new("seq");
