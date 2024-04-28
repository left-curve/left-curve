use {
    cw_std::{Addr, Hash, StdError},
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("VM error: {0}")]
    Vm(String),

    #[error("DB error: {0}")]
    Db(String),

    #[error("Merkle proof is not support for `/app` query; use `/store` instead")]
    ProofNotSupported,

    #[error("The sender does not have permission to perform this action")]
    Unauthorized,

    #[error("Incorrect block height! expecting: {expect}, actual: {actual}")]
    IncorrectBlockHeight {
        expect: u64,
        actual: u64,
    },

    #[error("Owner account is not set")]
    OwnerNotSet,

    #[error("Sender is not the owner! sender: {sender}, owner: {owner}")]
    NotOwner {
        sender: Addr,
        owner:  Addr,
    },

    #[error("Admin account is not set")]
    AdminNotSet,

    #[error("Sender is not the admin! sender: {sender}, admin: {admin}")]
    NotAdmin {
        sender: Addr,
        admin:  Addr,
    },

    #[error("Wasm byte code with hash `{hash}` already exists")]
    CodeExists {
        hash: Hash,
    },

    #[error("Account with address `{address}` already exists")]
    AccountExists {
        address: Addr,
    },

    #[error("Code hash is not allowed as IBC client: `{code_hash}`")]
    NotAllowedClient {
        code_hash: Hash,
    },
}

impl AppError {
    pub fn incorrect_block_height(expect: u64, actual: u64) -> Self {
        Self::IncorrectBlockHeight { expect, actual }
    }

    pub fn not_owner(sender: Addr, owner: Addr) -> Self {
        Self::NotOwner { sender, owner }
    }

    pub fn not_admin(sender: Addr, admin: Addr) -> Self {
        Self::NotAdmin { sender, admin }
    }

    pub fn code_exists(hash: Hash) -> Self {
        Self::CodeExists { hash }
    }

    pub fn account_exists(address: Addr) -> Self {
        Self::AccountExists { address }
    }

    pub fn not_allowed_client(code_hash: Hash) -> Self {
        Self::NotAllowedClient { code_hash }
    }
}

pub type AppResult<T> = std::result::Result<T, AppError>;
