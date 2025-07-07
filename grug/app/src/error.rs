use {
    grug_types::{Addr, Hash256, StdError},
    std::{
        collections::HashMap,
        sync::{MutexGuard, PoisonError},
    },
    thiserror::Error,
};

#[derive(Clone, Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("VM error: {0}")]
    Vm(String),

    #[error("DB error: {0}")]
    Db(String),

    #[error("proposal preparer error: {0}")]
    PrepareProposal(String),

    #[error("indexer error: {0}")]
    Indexer(#[from] IndexerError),

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
#[derive(Clone, Debug, Error)]
pub enum IndexerError {
    #[error("indexer is already running")]
    AlreadyRunning,

    #[error("indexer is not running")]
    NotRunning,

    #[error("storage error: {0}")]
    Storage(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("hook error: {0}")]
    Hook(String),

    #[error("multiple errors: {0:?}")]
    Multiple(Vec<String>),

    #[error("generic indexer error: {0}")]
    Generic(String),

    #[error("mutex for the indexers is poisoned")]
    MutexPoisoned,

    #[error("rwlock for the indexers is poisoned")]
    RwlockPoisoned,
}

impl From<std::io::Error> for IndexerError {
    fn from(err: std::io::Error) -> Self {
        IndexerError::Io(err.to_string())
    }
}

impl From<PoisonError<MutexGuard<'_, HashMap<u64, bool>>>> for IndexerError {
    fn from(_: PoisonError<MutexGuard<'_, HashMap<u64, bool>>>) -> Self {
        Self::MutexPoisoned
    }
}

pub type AppResult<T> = core::result::Result<T, AppError>;
