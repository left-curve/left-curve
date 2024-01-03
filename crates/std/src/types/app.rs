use {
    crate::{Addr, Hash, Message},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub code_hash: Hash,
    pub admin:     Option<Addr>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BlockInfo {
    pub height:    u64,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GenesisState {
    pub chain_id: String,
    pub msgs:     Vec<Message>,
}
