use {
    crate::{Addr, Coins, Hash, Timestamp, Uint64},
    borsh::{BorshDeserialize, BorshSerialize},
};

/// This is a union of all context types. When doing a Wasm function call, the
/// host constructs this, serialize to bytes, and pass it to the Wasm module.
#[derive(BorshSerialize, BorshDeserialize)]
pub struct Context {
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Option<Addr>,
    pub funds:           Option<Coins>,
    pub simulate:        Option<bool>,
}
