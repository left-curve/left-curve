use {
    grug_types::{Addr, Hash256, StdError},
    std::{
        collections::HashMap,
        sync::{MutexGuard, PoisonError},
    },
};

#[grug_macros::backtrace]
#[derive(Clone)]
pub enum AppError {
    #[error(transparent)]
    Std(StdError),

    #[error(transparent)]
    Indexer(IndexerError),

    #[error("VM error: {error}")]
    Vm { error: String },

    #[error("DB error: {error}")]
    Db { error: String },

    #[error("proposal preparer error: {error}")]
    PrepareProposal { error: String },

    #[error("contract returned error! address: {address}, method: {name}, msg: {msg}")]
    Guest {
        address: Addr,
        name: &'static str,
        msg: String,
    },

    #[error("merkle proof is not supported for `/app` query; use `/store` instead")]
    ProofNotSupported,

    #[error("simulating a transaction at past block height is not supported")]
    PastHeightNotSupported,

    #[error("sender does not have permission to perform this action")]
    Unauthorized,

    #[error("incorrect block height! expecting: {expect}, actual: {actual}")]
    IncorrectBlockHeight { expect: u64, actual: u64 },

    #[error("sender is not the owner! sender: {sender}, owner: {owner}")]
    NotOwner { sender: Addr, owner: Addr },

    #[error("admin account is not set")]
    AdminNotSet,

    #[error("sender is not the admin! sender: {sender}, admin: {admin}")]
    NotAdmin { sender: Addr, admin: Addr },

    #[error("code with hash `{code_hash}` already exists")]
    CodeExists { code_hash: Hash256 },

    #[error("account with address `{address}` already exists")]
    AccountExists { address: Addr },

    #[error("max message depth exceeded")]
    ExceedMaxMessageDepth,
}

/// Dedicated error type for indexer operations
#[grug_macros::backtrace]
#[derive(Clone)]
pub enum IndexerError {
    #[error("indexer is already running")]
    AlreadyRunning,

    #[error("indexer is not running")]
    NotRunning,

    #[error("I/O error: {error}")]
    Io { error: String },

    #[error("storage error: {error}")]
    Storage { error: String },

    #[error("database error: {error}")]
    Database { error: String },

    #[error("serialization error: {error}")]
    Serialization { error: String },

    #[error("configuration error: {error}")]
    Config { error: String },

    #[error("hook error: {error}")]
    Hook { error: String },

    #[error("multiple errors: {errors:?}")]
    Multiple { errors: Vec<String> },

    #[error("generic indexer error: {error}")]
    Generic { error: String },

    #[error("mutex for the indexers is poisoned")]
    MutexPoisoned,

    #[error("rwlock for the indexers is poisoned")]
    RwlockPoisoned,
}

impl From<PoisonError<MutexGuard<'_, HashMap<u64, bool>>>> for IndexerError {
    fn from(_: PoisonError<MutexGuard<'_, HashMap<u64, bool>>>) -> Self {
        Self::mutex_poisoned()
    }
}

pub type AppResult<T> = core::result::Result<T, AppError>;
