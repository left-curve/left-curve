use {
    grug_app::AppError,
    grug_types::{Backtraceable, BacktracedError, StdError},
};

#[grug_macros::backtrace]
pub enum IndexerError {
    #[error("sea_orm error: {0}")]
    #[backtrace(new)]
    SeaOrm(sea_orm::error::DbErr),

    #[error("anyhow error: {0}")]
    Anyhow(anyhow::Error),

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
    #[backtrace(new)]
    Persistence(indexer_disk_saver::error::Error),

    #[error("hooks error: {error}")]
    Hooks { error: String },

    #[error(transparent)]
    #[backtrace(new)]
    SerdeJson(serde_json::Error),

    #[error("parse error: {0}")]
    #[backtrace(new)]
    Parse(std::num::ParseIntError),

    #[error(transparent)]
    #[backtrace(new)]
    Sqlx(sqlx::Error),
}

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

impl From<IndexerError> for AppError {
    fn from(err: IndexerError) -> Self {
        let indexer_error = match err {
            IndexerError::SeaOrm(e) => parse_error!(Database, e),
            IndexerError::Anyhow(e) => parse_error!(Generic, e),
            IndexerError::Join(e) => parse_error!(Generic, e),
            IndexerError::Indexing { error, backtrace } => parse_error!(Generic, error, backtrace),
            IndexerError::Poison { error, backtrace } => parse_error!(Generic, error, backtrace),
            IndexerError::Runtime { error, backtrace } => parse_error!(Generic, error, backtrace),
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
            IndexerError::Sqlx(e) => parse_error!(Database, e),
        };

        let bt = indexer_error.backtrace();
        AppError::Indexer(BacktracedError::new_with_bt(indexer_error, bt))
    }
}

impl From<IndexerError> for grug_app::IndexerError {
    fn from(err: IndexerError) -> Self {
        match err {
            IndexerError::SeaOrm(e) => parse_error!(Database, e),
            IndexerError::Anyhow(e) => parse_error!(Generic, e),
            IndexerError::Join(e) => parse_error!(Generic, e),
            IndexerError::Indexing { error, backtrace } => parse_error!(Generic, error, backtrace),
            IndexerError::Poison { error, backtrace } => parse_error!(Generic, error, backtrace),
            IndexerError::Runtime { error, backtrace } => parse_error!(Generic, error, backtrace),
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
            IndexerError::Sqlx(e) => parse_error!(Database, e),
        }
    }
}

impl<T> From<std::sync::PoisonError<T>> for IndexerError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        IndexerError::poison(err.to_string())
    }
}

pub type Result<T> = core::result::Result<T, IndexerError>;

#[macro_export]
macro_rules! bail {
    ($variant:path, $msg:expr) => {
        return Err($variant($msg.into()).into());
    };
    ($($arg:tt)*) => {
        return Err($crate::error::IndexerError::indexing(format!($($arg)*)));
    };
}
