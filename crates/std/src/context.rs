use {
    crate::{Addr, Coins, Hash, Storage, Timestamp, Uint64},
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

/// A context that contians an immutable store. The contract is allowed to read
/// data from the store, but not write to it. This is used in query calls.
pub struct ImmutableCtx<'a> {
    pub store:           &'a dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

/// A context that contains a mutable store. This is used for entry points where
/// the contract is allowed to mutate the state, such as instantiate and execute.
pub struct MutableCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
    pub funds:           Coins,
}

/// Sudo context is a state-mutable context. This is used when a contract is
/// called by the chain, instead of by a message sent by another account.
/// Therefore, compared to `MutableCtx`, it lacks the `sender` and `funds` fields.
///
/// The name is derived from the "sudo" entry point in the vanilla CosmWasm.
/// There isn't such an entry point in CWD, but we keep the name nonetheless.
pub struct SudoCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

/// Similar to `SudoCtx`, but with an additional parameter `simulate` which
/// designates whether the contract call is done in the simulation mode (e.g.
/// during the `CheckTx` ABCI call).
///
/// This is used in the `before_tx` and `after_tx` entry points, whose primary
/// purpose is to authenticate transactions, hence the name.
///
/// The typical use of the `simulate` parameter is to skip certain authentication
/// steps (e.g. verifying a cryptographic signature) if it's in simulation mode.
pub struct AuthCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub simulate:        bool,
}
