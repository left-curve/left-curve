use {
    crate::{
        hooks::{HookMsg, HookQuery, HookQueryResponse},
        incremental_merkle_tree::IncrementalMerkleTree,
    },
    grug::{Addr, Hash256},
};

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Address of the mailbox contract.
    pub mailbox: Addr,
}

#[grug::derive(Serde)]
pub enum ExecuteMsg {
    /// Required Hyperlane hook interface.
    Hook(HookMsg),
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the mailbox contract address.
    #[returns(Addr)]
    Mailbox {},
    /// Query the Merkle tree.
    #[returns(IncrementalMerkleTree)]
    Tree {},
    /// Required Hyperlane hook interface.
    #[returns(HookQueryResponse)]
    Hook(HookQuery),
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
