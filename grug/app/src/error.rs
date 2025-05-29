use {
    grug_types::{Addr, Hash256, StdError},
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
    Indexer(String),

    #[error("indexer error: {0}")]
    IndexerHttpd(String),

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

pub type AppResult<T> = core::result::Result<T, AppError>;
