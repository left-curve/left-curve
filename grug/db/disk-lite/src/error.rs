use {
    crate::PendingData,
    error_backtrace::Backtraceable,
    grug_app::AppError,
    grug_types::StdError,
    std::sync::{PoisonError, RwLockReadGuard, RwLockWriteGuard},
};

#[error_backtrace::backtrace]
pub enum DbError {
    #[error(transparent)]
    Std(StdError),

    #[error(transparent)]
    #[backtrace(new)]
    RocksDb(rocksdb::Error),

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

impl<'a> From<PoisonError<RwLockReadGuard<'a, Option<PendingData>>>> for DbError {
    fn from(_: PoisonError<RwLockReadGuard<'a, Option<PendingData>>>) -> Self {
        Self::pending_data_poisoned()
    }
}

impl<'a> From<PoisonError<RwLockWriteGuard<'a, Option<PendingData>>>> for DbError {
    fn from(_: PoisonError<RwLockWriteGuard<'a, Option<PendingData>>>) -> Self {
        Self::pending_data_poisoned()
    }
}

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        let err = err.into_generic_backtraced_error();
        AppError::Db {
            error: err.to_string(),
            backtrace: err.backtrace(),
        }
    }
}

pub type DbResult<T> = Result<T, DbError>;
