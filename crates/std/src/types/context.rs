use {
    crate::{Addr, Coins, Event, GenericResult, Hash, Storage, Timestamp, Uint64},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
};

/// The context passed by the host to the Wasm module whenever an entry point is
/// called. The module then converts this to Instantiate/Execute/Query or other
/// contexts for easy usage by the contract programmer.
///
/// Some fields may be optional depending on which entry point is called.
/// For example, for queries there is no sender, because queries are not part of
/// a transaction.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Option<Addr>,
    pub funds:           Option<Coins>,
    pub simulate:        Option<bool>,
    pub submsg_result:   Option<GenericResult<Vec<Event>>>,
}

pub struct InstantiateCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
    pub funds:           Coins,
}

pub struct ExecuteCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
    pub funds:           Coins,
}

pub struct QueryCtx<'a> {
    pub store:           &'a dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct MigrateCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
}

pub struct ReplyCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub submsg_result:   GenericResult<Vec<Event>>,
}

pub struct ReceiveCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub sender:          Addr,
    pub funds:           Coins,
}

pub struct BeforeBlockCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct AfterBlockCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct BeforeTxCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub simulate:        bool,
}

pub struct AfterTxCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
    pub simulate:        bool,
}

pub struct TransferCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct IbcClientCreateCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct IbcClientUpdateCtx<'a> {
    pub store:           &'a mut dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}

pub struct IbcClientVerifyCtx<'a> {
    pub store:           &'a dyn Storage,
    pub chain_id:        String,
    pub block_height:    Uint64,
    pub block_timestamp: Timestamp,
    pub block_hash:      Hash,
    pub contract:        Addr,
}
