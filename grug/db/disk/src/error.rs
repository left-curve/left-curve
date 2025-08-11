use {
    crate::PendingData,
    grug_app::AppError,
    grug_types::{Backtraceable, StdError},
    std::sync::{PoisonError, RwLockReadGuard, RwLockWriteGuard},
};

#[grug_macros::backtrace]
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

    #[error("requested version ({version}) is newer than the latest version ({latest_version})")]
    VersionTooNew { version: u64, latest_version: u64 },

    #[error(
        "requested version ({version}) is older than the oldest available version ({oldest_version})"
    )]
    VersionTooOld { version: u64, oldest_version: u64 },
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

pub type DbResult<T> = core::result::Result<T, DbError>;
