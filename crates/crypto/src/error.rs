use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error(transparent)]
    Signature(#[from] signature::Error),

    #[error("data is of incorrect length: expecting {expect}, found {actual}")]
    IncorrectLength { expect: usize, actual: usize },
}

impl CryptoError {
    pub fn incorrect_length(expect: usize, actual: usize) -> Self {
        Self::IncorrectLength { expect, actual }
    }
}

pub type CryptoResult<T> = std::result::Result<T, CryptoError>;
