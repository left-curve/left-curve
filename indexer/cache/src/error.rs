use {grug_app::AppError, grug_types::StdError, thiserror::Error};

#[derive(Debug, Error)]
pub enum IndexerError {
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

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error("parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),
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
