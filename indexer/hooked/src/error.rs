use thiserror::Error;

/// Error types for the HookedIndexer system
#[derive(Debug, Error)]
pub enum HookedIndexerError {
    /// Indexer is already running
    #[error("Indexer is already running")]
    AlreadyRunning,

    /// Indexer is not running
    #[error("Indexer is not running")]
    NotRunning,

    /// Error from a hook
    #[error("Hook error: {0}")]
    Hook(String),

    /// Multiple errors occurred
    #[error("Multiple errors: {0:?}")]
    Multiple(Vec<String>),

    /// Generic error with message
    #[error("{0}")]
    Generic(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<std::io::Error> for HookedIndexerError {
    fn from(err: std::io::Error) -> Self {
        HookedIndexerError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for HookedIndexerError {
    fn from(err: serde_json::Error) -> Self {
        HookedIndexerError::Serialization(err.to_string())
    }
}

impl From<std::convert::Infallible> for HookedIndexerError {
    fn from(_: std::convert::Infallible) -> Self {
        // This should never happen since Infallible can never be constructed
        unreachable!("Infallible error should never occur")
    }
}

/// Result type alias for HookedIndexer operations
pub type Result<T> = std::result::Result<T, HookedIndexerError>;

/// Helper trait for converting errors into HookedIndexerError
pub trait IntoHookedIndexerError {
    fn into_hooked_indexer_error(self) -> HookedIndexerError;
}

impl<E: ToString + std::fmt::Debug> IntoHookedIndexerError for E {
    fn into_hooked_indexer_error(self) -> HookedIndexerError {
        HookedIndexerError::Generic(self.to_string())
    }
}

impl From<HookedIndexerError> for grug_app::AppError {
    fn from(err: HookedIndexerError) -> Self {
        grug_app::AppError::Indexer(err.to_string())
    }
}
