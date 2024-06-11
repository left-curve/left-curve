use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error(transparent)]
    Signature(#[from] signature::Error),

    #[error("data is of incorrect length: expecting {expect}, found {actual}")]
    IncorrectLength { expect: usize, actual: usize },

    #[error("invalid recovery id {recovery_id}")]
    InvalidRecoveryId { recovery_id: u8 },
}

pub type CryptoResult<T> = core::result::Result<T, CryptoError>;
