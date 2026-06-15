use {dango_app::AppError, dango_primitives::StdError};

#[dango_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("join error: {0}")]
    #[backtrace(new)]
    Join(tokio::task::JoinError),

    #[error("mutex is poisoned: {error}")]
    MutexPoisoned { error: String },

    #[error(transparent)]
    #[backtrace(new)]
    StripPrefixError(std::path::StripPrefixError),

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
    Persistence(dango_disk_saver::error::Error),

    #[error(transparent)]
    #[backtrace(new)]
    SerdeJson(serde_json::Error),

    #[error("parse error: {0}")]
    #[backtrace(new)]
    Parse(std::num::ParseIntError),

    #[error("s3 error: {error}")]
    #[backtrace(new)]
    S3 { error: String },

    #[error("s3 upload failed after retries: block {block_height}, key {key}")]
    #[backtrace(new)]
    S3UploadFailed { block_height: u64, key: String },
}

pub type Result<T> = core::result::Result<T, IndexerError>;

macro_rules! parse_error {
    ($variant:ident, $e:expr) => {
        dango_app::IndexerError::$variant {
            error: $e.to_string(),
            backtrace: $e.backtrace,
        }
    };
    ($variant:ident, $e:expr, $bt:expr) => {
        dango_app::IndexerError::$variant {
            error: $e.to_string(),
            backtrace: $bt,
        }
    };
}

impl From<IndexerError> for dango_app::IndexerError {
    fn from(err: IndexerError) -> Self {
        match err {
            IndexerError::StripPrefixError(e) => parse_error!(Generic, e),
            IndexerError::Join(e) => parse_error!(Generic, e),
            // IndexerError::Indexing { error, backtrace } => parse_error!(Generic, error, backtrace),
            IndexerError::MutexPoisoned { error, backtrace } => {
                parse_error!(Generic, error, backtrace)
            },
            IndexerError::ByteStream { error, backtrace } => {
                parse_error!(Generic, error, backtrace)
            },
            IndexerError::TryFromInt(e) => parse_error!(Generic, e),
            IndexerError::App(be) => {
                // For App errors, just wrap as generic since it's already processed
                dango_app::IndexerError::Generic {
                    error: "nested app error".to_string(),
                    backtrace: be.backtrace,
                }
            },
            IndexerError::Std(e) => parse_error!(Generic, e),
            IndexerError::Io(e) => parse_error!(Io, e),
            IndexerError::Persist(e) => parse_error!(Io, e),
            IndexerError::Persistence(e) => parse_error!(Storage, e),
            IndexerError::SerdeJson(e) => parse_error!(Serialization, e),
            IndexerError::Parse(e) => parse_error!(Generic, e),
            IndexerError::S3 { error, backtrace } => parse_error!(Generic, error, backtrace),
            IndexerError::S3UploadFailed {
                block_height,
                key,
                backtrace,
            } => dango_app::IndexerError::Generic {
                error: format!("s3 upload failed after retries: block {block_height}, key {key}"),
                backtrace,
            },
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
