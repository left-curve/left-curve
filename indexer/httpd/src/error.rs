use {indexer_sql::error::IndexerError, std::io};

#[error_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    #[backtrace(new)]
    Io(io::Error),

    #[error(transparent)]
    Indexer(IndexerError),
}
