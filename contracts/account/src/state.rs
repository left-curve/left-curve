use {
    crate::PublicKey,
    grug_storage::{Counter, Item, Set},
    grug_types::Hash,
};

/// The account's public key.
pub const PUBLIC_KEY: Item<PublicKey> = Item::new("pk");

/// The account's sequence number, also known as "nonce" in Ethereum.
pub const SEQUENCE: Counter<u32> = Counter::new("seq");

/// Unordered transactions that have been included in a block.
pub const UNORDERED_TXS: Set<(u128, &Hash)> = Set::new("unordered_txs");

/// json key for account data in tx data
pub const DATA_ACCOUNT_KEY: &str = "account";
