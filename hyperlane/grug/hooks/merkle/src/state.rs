use {
    grug::{Addr, Item},
    hyperlane_types::IncrementalMerkleTree,
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

pub const MERKLE_TREE: Item<IncrementalMerkleTree> = Item::new("merkle_tree");
