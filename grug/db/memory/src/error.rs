use {grug_app::AppError, grug_types::StdError, thiserror::Error};

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("cannot flush when changeset is already set")]
    ChangeSetAlreadySet,

    #[error("cannot commit when changeset is not yet set")]
    ChangeSetNotSet,
}

impl From<DbError> for AppError {
    fn from(err: DbError) -> Self {
        AppError::Db(err.to_string())
    }
}

pub type DbResult<T> = core::result::Result<T, DbError>;
