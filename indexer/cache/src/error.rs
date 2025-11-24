use {grug_app::AppError, grug_types::StdError};

#[error_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("join error: {0}")]
    #[backtrace(new)]
    Join(tokio::task::JoinError),

    #[error("indexing error: {error}")]
    Indexing { error: String },

    #[error("mutex poison error: {error}")]
    Poison { error: String },

    #[error("runtime error: {error}")]
    Runtime { error: String },

    #[error(transparent)]
    #[backtrace(new)]
    StripPrefixError(std::path::StripPrefixError),

    #[cfg(feature = "s3")]
    #[error("byte stream error: {error}")]
    #[backtrace(new)]
    ByteStream { error: String },

    #[error(transparent)]
    #[backtrace(new)]
    TryFromInt(std::num::TryFromIntError),

    #[error(transparent)]
    App(AppError),

    #[error(transparent)]
    Std(StdError),

    #[error(transparent)]
    #[backtrace(new)]
    Io(std::io::Error),

    #[error(transparent)]
    #[backtrace(new)]
    Persist(tempfile::PersistError),

    #[error(transparent)]
    Persistence(indexer_disk_saver::error::Error),

    #[error("hooks error: {error}")]
    Hooks { error: String },

    #[error(transparent)]
    #[backtrace(new)]
    SerdeJson(serde_json::Error),

    #[error("parse error: {0}")]
    #[backtrace(new)]
    Parse(std::num::ParseIntError),

    #[error("s3 error: {error}")]
    #[backtrace(new)]
    S3 { error: String },
}

pub type Result<T> = core::result::Result<T, IndexerError>;

macro_rules! parse_error {
    ($variant:ident, $e:expr) => {
        grug_app::IndexerError::$variant {
            error: $e.to_string(),
            backtrace: $e.backtrace,
        }
    };
    ($variant:ident, $e:expr, $bt:expr) => {
        grug_app::IndexerError::$variant {
            error: $e.to_string(),
            backtrace: $bt,
        }
    };
}

impl From<IndexerError> for grug_app::IndexerError {
    fn from(err: IndexerError) -> Self {
        match err {
            IndexerError::StripPrefixError(e) => parse_error!(Generic, e),
            IndexerError::Join(e) => parse_error!(Generic, e),
            IndexerError::Indexing { error, backtrace } => parse_error!(Generic, error, backtrace),
            IndexerError::Poison { error, backtrace } => parse_error!(Generic, error, backtrace),
            IndexerError::Runtime { error, backtrace } => parse_error!(Generic, error, backtrace),
            #[cfg(feature = "s3")]
            IndexerError::ByteStream { error, backtrace } => {
                parse_error!(Generic, error, backtrace)
            },
            IndexerError::TryFromInt(e) => parse_error!(Generic, e),
            IndexerError::App(be) => {
                // For App errors, just wrap as generic since it's already processed
                grug_app::IndexerError::Generic {
                    error: "nested app error".to_string(),
                    backtrace: be.backtrace,
                }
            },
            IndexerError::Std(e) => parse_error!(Generic, e),
            IndexerError::Io(e) => parse_error!(Io, e),
            IndexerError::Persist(e) => parse_error!(Io, e),
            IndexerError::Persistence(e) => parse_error!(Storage, e),
            IndexerError::Hooks { error, backtrace } => parse_error!(Hook, error, backtrace),
            IndexerError::SerdeJson(e) => parse_error!(Serialization, e),
            IndexerError::Parse(e) => parse_error!(Generic, e),
            IndexerError::S3 { error, backtrace } => parse_error!(Generic, error, backtrace),
        }
    }
}

#[macro_export]
macro_rules! bail {
    ($variant:path, $msg:expr) => {
        return Err($variant($msg.into()).into());
    };
    ($($arg:tt)*) => {
        return Err($crate::error::IndexerError::Indexing(format!($($arg)*)));
    };
}
