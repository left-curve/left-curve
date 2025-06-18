use {
    grug_app::AppError,
    std::sync::{PoisonError, RwLockReadGuard, RwLockWriteGuard},
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Std(#[from] grug_types::StdError),

    #[error(transparent)]
    RocksDb(#[from] rocksdb::Error),

    #[error("cannot flush when the in-memory write batch is already set")]
    PendingDataAlreadySet,

    #[error("cannot commit when the in-memory write batch is not set")]
    PendingDataNotSet,

    #[error("rwlock for the write batch is poisoned")]
    PendingDataPoisoned,

    #[error("requested version ({requested}) does not equal the DB version ({db_version})")]
    IncorrectVersion { db_version: u64, requested: u64 },

    #[error("state proof is not supported")]
    ProofUnsupported,
}

impl<'a, T> From<PoisonError<RwLockReadGuard<'a, T>>> for DbError {
    fn from(_: PoisonError<RwLockReadGuard<'a, T>>) -> Self {
        Self::PendingDataPoisoned
    }
}

impl<'a, T> From<PoisonError<RwLockWriteGuard<'a, T>>> for DbError {
    fn from(_: PoisonError<RwLockWriteGuard<'a, T>>) -> Self {
        Self::PendingDataPoisoned
    }
}

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        AppError::Db(err.to_string())
    }
}

pub type DbResult<T> = Result<T, DbError>;
