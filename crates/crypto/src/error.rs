use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error(transparent)]
    Signature(#[from] signature::Error),

    #[error("data is of incorrect length: expecting {expect}, found {actual}")]
    IncorrectLength { expect: usize, actual: usize },

    #[error("data is of incorrect length: expecting on of {expect:?}, found {actual}")]
    IncorrectLengths { expect: Vec<usize>, actual: usize },

    #[error("invalid recovery id {recovery_id}")]
    InvalidRecoveryId { recovery_id: u8 },

    #[error("array exceeds maximum length, max {max_length}, found {actual_length}")]
    ExceedsMaximumLength {
        max_length: usize,
        actual_length: usize,
    },
}

pub type CryptoResult<T> = core::result::Result<T, CryptoError>;
