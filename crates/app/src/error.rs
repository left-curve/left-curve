use {
    cw_db::DbError,
    cw_std::{Addr, Hash, StdError},
    cw_vm::VmError,
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Vm(#[from] VmError),

    #[error(transparent)]
    Db(#[from] DbError),

    #[error("Merkle proof is not support for `/app` query; use `/store` instead")]
    ProofNotSupported,

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
}

impl AppError {
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
}

pub type AppResult<T> = std::result::Result<T, AppError>;
