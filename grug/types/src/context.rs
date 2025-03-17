// Skip formatting this entire file
// https://stackoverflow.com/questions/59247458/is-there-a-stable-way-to-tell-rustfmt-to-skip-an-entire-file
#![cfg_attr(rustfmt, rustfmt::skip)]

use {
    crate::{Addr, Api, BlockInfo, Coins, QuerierWrapper, Storage},
    borsh::{BorshDeserialize, BorshSerialize},
};

/// This is a union of all context types. When doing a Wasm function call, the
/// host constructs this, serialize to bytes, and pass it to the Wasm module.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Context {
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Option<Addr>,
    pub funds:    Option<Coins>,
    pub mode:     Option<AuthMode>,
}

/// A context that contains an immutable store. The contract is allowed to read
/// data from the store, but not write to it. This is used in query calls.
pub struct ImmutableCtx<'a> {
    pub storage:  &'a dyn Storage,
    pub api:      &'a dyn Api,
    pub querier:  QuerierWrapper<'a>,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
}

/// A context that contains a mutable store. This is used for entry points where
/// the contract is allowed to mutate the state, such as instantiate and execute.
pub struct MutableCtx<'a> {
    pub storage:  &'a mut dyn Storage,
    pub api:      &'a dyn Api,
    pub querier:  QuerierWrapper<'a>,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Addr,
    pub funds:    Coins,
}

/// Sudo context is a state-mutable context. This is used when a contract is
/// called by the chain, instead of by a message sent by another account.
/// Therefore, compared to `MutableCtx`, it lacks the `sender` and `funds` fields.
///
/// The name is derived from the "sudo" entry point in the vanilla CosmWasm.
/// There isn't such an entry point in Grug, but we keep the name nonetheless.
pub struct SudoCtx<'a> {
    pub storage:  &'a mut dyn Storage,
    pub api:      &'a dyn Api,
    pub querier:  QuerierWrapper<'a>,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
}

/// Similar to `SudoCtx`, but with an additional parameter `simulate` which
/// designates whether the contract call is done in the simulation mode (e.g.
/// during the `CheckTx` ABCI call).
///
/// This is used in the `authenticate` and `backrun` entry points, whose primary
/// purpose is to authenticate transactions, hence the name.
///
/// The typical use of the `simulate` parameter is to skip certain authentication
/// steps (e.g. verifying a cryptographic signature) if it's in simulation mode.
pub struct AuthCtx<'a> {
    pub storage:  &'a mut dyn Storage,
    pub api:      &'a dyn Api,
    pub querier:  QuerierWrapper<'a>,
    pub chain_id: String,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub mode:     AuthMode,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    Simulate,
    Check,
    Finalize
}
