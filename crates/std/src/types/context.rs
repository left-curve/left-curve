use crate::BlockInfo;

use {
    crate::{Addr, Storage},
    serde::{Deserialize, Serialize},
};

/// The context passed by the host to the Wasm module whenever an entry point is
/// called. The module then converts this to Instantiate/Execute/Query or other
/// contexts for easy usage by the contract programmer.
///
/// Some fields may be optional depending on which entry point is called.
/// For example, for queries there is no sender, because queries are not part of
/// a transaction.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Option<Addr>,
    pub simulate: Option<bool>,
}

// BlockTxCtx doesn't have a `sender` field, because only messages have senders,
// but a before_tx call isn't triggered by a message (it's triggered by a tx) so
// sender doesn't apply.
pub struct BeforeTxCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub simulate: bool,
}

pub struct InstantiateCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Addr,
}

pub struct ExecuteCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Addr,
}

pub struct QueryCtx<'a> {
    pub store:    &'a dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
}
