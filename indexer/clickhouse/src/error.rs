use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("indexing error: {0}")]
    Indexing(String),

    #[error("runtime error: {0}")]
    Runtime(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
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
