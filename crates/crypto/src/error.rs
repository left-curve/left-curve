use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error(transparent)]
    Signature(#[from] signature::Error),

    #[error(transparent)]
    Bls(#[from] bls_signatures::Error),

    #[error("data is of incorrect length: expecting {expect}, found {actual}")]
    IncorrectLength { expect: usize, actual: usize },

    #[error("invalid recovery id {id}")]
    InvalidRecoveryId { id: u8 },
}

impl CryptoError {
    pub fn incorrect_length(expect: usize, actual: usize) -> Self {
        Self::IncorrectLength { expect, actual }
    }

    pub fn invalid_recovery_id(id: u8) -> Self {
        Self::InvalidRecoveryId { id }
    }
}

pub type CryptoResult<T> = std::result::Result<T, CryptoError>;
