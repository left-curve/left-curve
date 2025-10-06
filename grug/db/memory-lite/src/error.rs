use {
    grug_app::AppError,
    grug_types::{Backtraceable, StdError},
};

#[grug_macros::backtrace]
pub enum DbError {
    #[error(transparent)]
    Std(StdError),

    #[error("cannot flush when changeset is already set")]
    ChangeSetAlreadySet,

    #[error("cannot commit when changeset is not yet set")]
    ChangeSetNotSet,

    #[error("state proof is not supported")]
    ProofUnsupported,

    #[error("requested version ({requested}) does not equal the DB version ({db_version})")]
    IncorrectVersion { db_version: u64, requested: u64 },
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
