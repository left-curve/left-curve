use {
    crate::{Addr, BlockInfo, Coins, Storage},
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
    pub funds:    Option<Coins>,
    pub simulate: Option<bool>,
}

pub struct InstantiateCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Addr,
    pub funds:    Coins,
}

pub struct ExecuteCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Addr,
    pub funds:    Coins,
}

pub struct QueryCtx<'a> {
    pub store:    &'a dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
}

pub struct MigrateCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Addr,
}

pub struct BeforeTxCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub simulate: bool,
}

pub struct TransferCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
}

pub struct ReceiveCtx<'a> {
    pub store:    &'a mut dyn Storage,
    pub block:    BlockInfo,
    pub contract: Addr,
    pub sender:   Addr,
    pub funds:    Coins,
}
