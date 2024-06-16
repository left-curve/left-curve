use {
    crate::PublicKey,
    grug::{Incrementor, Item},
};

/// The account's public key.
pub const PUBLIC_KEY: Item<PublicKey> = Item::new("pk");

/// The account's sequence number, also known as "nonce" in Ethereum.
pub const SEQUENCE: Incrementor<u32> = Incrementor::new("seq");
