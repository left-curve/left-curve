use {
    grug_storage::{Counter, Item},
    grug_types::Binary,
};

/// The Secp256k1 public key associated with the account.
pub const PUBLIC_KEY: Item<Binary> = Item::new("pk");

/// The account's sequence number, also known as "nonce" in Ethereum.
pub const SEQUENCE: Counter<u32> = Counter::new("seq");
