use {
    crate::{Indexer, IndexerError},
    std::{
        convert::Infallible,
        fmt::{self, Display},
    },
};

/// This is a null indexer that does nothing.
#[derive(Debug, Clone, Default)]
pub struct NullIndexer;

impl Indexer for NullIndexer {}

/// An error type that is never encountered.
/// Used in conjunction with [`NullIndexer`](crate::NullIndexer).
#[derive(Debug)]
pub struct NullIndexerError(Infallible);

impl Display for NullIndexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<NullIndexerError> for IndexerError {
    fn from(err: NullIndexerError) -> Self {
        IndexerError::generic(err.to_string())
    }
}
