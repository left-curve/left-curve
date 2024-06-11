use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error(transparent)]
    Signature(#[from] signature::Error),

    #[error("data is of incorrect length: expecting {expect}, found {actual}")]
    IncorrectLength { expect: usize, actual: usize },

    #[error("invalid recovery id: max {max}, found {actual}")]
    InvalidRecoveryId { max: u8, actual: u8 },
}

impl CryptoError {
    pub fn incorrect_length(expect: usize, actual: usize) -> Self {
        Self::IncorrectLength { expect, actual }
    }
    pub fn invalid_recovery_id(max: u8, actual: u8) -> Self {
        Self::InvalidRecoveryId { max, actual }
    }
}

pub type CryptoResult<T> = std::result::Result<T, CryptoError>;
