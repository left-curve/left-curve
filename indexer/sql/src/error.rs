use {grug_app::AppError, grug_types::StdError, thiserror::Error};

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("sea_orm error: {0}")]
    SeaOrm(#[from] sea_orm::error::DbErr),

    #[error("anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("indexing error: {0}")]
    Indexing(String),

    #[error("mutex poison error: {0}")]
    Poison(String),

    #[error("runtime error: {0}")]
    Runtime(String),

    #[error(transparent)]
    TryFromInt(#[from] std::num::TryFromIntError),

    #[error(transparent)]
    App(#[from] AppError),

    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Persist(#[from] tempfile::PersistError),

    #[error(transparent)]
    Persistence(#[from] indexer_disk_saver::error::Error),

    #[error("hooks error: {0}")]
    Hooks(String),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error("parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Acquire(#[from] tokio::sync::AcquireError),
}

impl From<IndexerError> for AppError {
    fn from(err: IndexerError) -> Self {
        AppError::Indexer(err.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for IndexerError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        IndexerError::Poison(format!("Poisoned mutex: {:?}", err))
    }
}

pub type Result<T> = core::result::Result<T, IndexerError>;

#[macro_export]
macro_rules! bail {
    ($variant:path, $msg:expr) => {
        return Err($variant($msg.into()).into());
    };
    ($($arg:tt)*) => {
        return Err($crate::error::IndexerError::Indexing(format!($($arg)*)));
    };
}
