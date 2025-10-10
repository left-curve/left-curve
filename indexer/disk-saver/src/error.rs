use grug_types::StdError;

#[grug_macros::backtrace]
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
