use {
    crate::{Addr, BlockInfo, Coins},
    borsh::{BorshDeserialize, BorshSerialize},
};

/// This is a union of all context types. When doing a Wasm function call, the
/// host constructs this, serialize to bytes, and pass it to the Wasm module.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Context {
    pub chain_id: String,
    pub block: BlockInfo,
    pub contract: Addr,
    pub sender: Option<Addr>,
    pub funds: Option<Coins>,
    pub simulate: Option<bool>,
}
