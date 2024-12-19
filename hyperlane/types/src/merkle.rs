use {
    crate::merkle_tree::MerkleTree,
    grug::{Addr, Coins, Hash256, HexBinary},
};

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    // Required Hyperlane hook interface.
    PostDispatch {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the mailbox contract address.
    #[returns(Addr)]
    Mailbox {},
    /// Query the Merkle tree.
    #[returns(MerkleTree)]
    Tree {},
    // Required Hyperlane hook interface.
    #[returns(Coins)]
    QuoteDispatch {
        raw_message: HexBinary,
        metadata: HexBinary,
    },
}

// ---------------------------------- events -----------------------------------

#[grug::derive(Serde)]
pub struct PostDispatch {
    pub message_id: Hash256,
    pub index: u128,
}

#[grug::derive(Serde)]
pub struct InsertedIntoTree {
    pub index: u128,
}
