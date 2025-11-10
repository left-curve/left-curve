use {error_backtrace::Backtraceable, grug_app::AppError, grug_types::StdError};

#[error_backtrace::backtrace]
#[derive(Debug, Clone, thiserror::Error)]
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

    #[error("requested version ({requested}) doesn't equal the current version ({current})")]
    IncorretVersion { requested: u64, current: u64 },
}

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        AppError::Db {
            error: err.error(),
            backtrace: err.backtrace(),
        }
    }
}

pub type DbResult<T> = core::result::Result<T, DbError>;
