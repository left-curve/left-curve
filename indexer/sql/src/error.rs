use {
    crate::pubsub::error::PubSubError, grug_app::AppError, grug_types::StdError, thiserror::Error,
};

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
    PubSubError(#[from] PubSubError),
}

impl From<IndexerError> for AppError {
    fn from(err: IndexerError) -> Self {
        let indexer_error = match err {
            IndexerError::SeaOrm(e) => grug_app::IndexerError::Database(e.to_string()),
            IndexerError::Anyhow(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::Join(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::Indexing(msg) => grug_app::IndexerError::Generic(msg),
            IndexerError::Poison(msg) => grug_app::IndexerError::Generic(msg),
            IndexerError::Runtime(msg) => grug_app::IndexerError::Generic(msg),
            IndexerError::TryFromInt(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::App(_) => {
                // For App errors, just wrap as generic since it's already processed
                grug_app::IndexerError::Generic("nested app error".to_string())
            },
            IndexerError::Std(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::Io(e) => grug_app::IndexerError::Io(e.to_string()),
            IndexerError::Persist(e) => grug_app::IndexerError::Io(e.to_string()),
            IndexerError::Persistence(e) => grug_app::IndexerError::Storage(e.to_string()),
            IndexerError::Hooks(msg) => grug_app::IndexerError::Hook(msg),
            IndexerError::SerdeJson(e) => grug_app::IndexerError::Serialization(e.to_string()),
            IndexerError::Parse(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::Sqlx(e) => grug_app::IndexerError::Database(e.to_string()),
            IndexerError::PubSubError(e) => grug_app::IndexerError::Generic(e.to_string()),
        };
        AppError::Indexer(indexer_error)
    }
}

impl From<IndexerError> for grug_app::IndexerError {
    fn from(err: IndexerError) -> Self {
        match err {
            IndexerError::SeaOrm(e) => grug_app::IndexerError::Database(e.to_string()),
            IndexerError::Anyhow(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::Join(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::Indexing(msg) => grug_app::IndexerError::Generic(msg),
            IndexerError::Poison(msg) => grug_app::IndexerError::Generic(msg),
            IndexerError::Runtime(msg) => grug_app::IndexerError::Generic(msg),
            IndexerError::TryFromInt(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::App(_) => {
                // For App errors, just wrap as generic since it's already processed
                grug_app::IndexerError::Generic("nested app error".to_string())
            },
            IndexerError::Std(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::Io(e) => grug_app::IndexerError::Io(e.to_string()),
            IndexerError::Persist(e) => grug_app::IndexerError::Io(e.to_string()),
            IndexerError::Persistence(e) => grug_app::IndexerError::Storage(e.to_string()),
            IndexerError::Hooks(msg) => grug_app::IndexerError::Hook(msg),
            IndexerError::SerdeJson(e) => grug_app::IndexerError::Serialization(e.to_string()),
            IndexerError::Parse(e) => grug_app::IndexerError::Generic(e.to_string()),
            IndexerError::Sqlx(e) => grug_app::IndexerError::Database(e.to_string()),
            IndexerError::PubSubError(e) => grug_app::IndexerError::Generic(e.to_string()),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for IndexerError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        IndexerError::Poison(err.to_string())
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
