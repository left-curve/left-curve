use grug_types::StdError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Persist(#[from] tempfile::PersistError),

    #[error(transparent)]
    Lzma(#[from] lzma_rs::error::Error),
}
