use {
    grug::{Addr, Item},
    hyperlane_types::merkle_tree::MerkleTree,
};

pub const MAILBOX: Item<Addr> = Item::new("mailbox");

pub const MERKLE_TREE: Item<MerkleTree> = Item::new("merkle_tree");
