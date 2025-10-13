use {error_backtrace::Backtraceable, grug_app::AppError, grug_types::StdError};

#[error_backtrace::backtrace]
#[derive(Debug, Clone, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Std(StdError),

    #[error("cannot flush when changeset is already set")]
    ChangeSetAlreadySet,

    #[error("cannot commit when changeset is not yet set")]
    ChangeSetNotSet,
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
