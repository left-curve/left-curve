use {
    grug_types::{Addr, Hash, StdError},
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    VM(#[from] VmError),

    #[error("DB error: {0}")]
    Db(String),

    #[error("Merkle proof is not support for `/app` query; use `/store` instead")]
    ProofNotSupported,

    #[error("sender does not have permission to perform this action")]
    Unauthorized,

    #[error("incorrect block height! expecting: {expect}, actual: {actual}")]
    IncorrectBlockHeight { expect: u64, actual: u64 },

    #[error("owner account is not set")]
    OwnerNotSet,

    #[error("sender is not the owner! sender: {sender}, owner: {owner}")]
    NotOwner { sender: Addr, owner: Addr },

    #[error("admin account is not set")]
    AdminNotSet,

    #[error("sender is not the admin! sender: {sender}, admin: {admin}")]
    NotAdmin { sender: Addr, admin: Addr },

    #[error("code with hash `{code_hash}` already exists")]
    CodeExists { code_hash: Hash },

    #[error("account with address `{address}` already exists")]
    AccountExists { address: Addr },

    #[error("code hash is not allowed as IBC client: `{code_hash}`")]
    NotAllowedClient { code_hash: Hash },
}

pub type AppResult<T> = core::result::Result<T, AppError>;

#[derive(Debug, Error)]
pub enum VmError {
    #[error("missing export function: {0}")]
    MissingExportFunction(String),
    #[error("{0}")]
    GenericError(String),
}
