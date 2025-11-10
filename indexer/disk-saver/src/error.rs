use grug_types::StdError;

#[error_backtrace::backtrace]
#[derive(Debug, thiserror::Error)]
pub enum Error {
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
    Lzma(lzma_rs::error::Error),
}
