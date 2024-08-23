use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error(transparent)]
    Signature(#[from] signature::Error),

    #[error("data is of incorrect length: expecting {expect}, found {actual}")]
    IncorrectLength { expect: usize, actual: usize },

    #[error("data is of incorrect length: expecting one of {expect:?}, found {actual}")]
    IncorrectLengths {
        expect: &'static [usize],
        actual: usize,
    },

    #[error("invalid recovery id {recovery_id}")]
    InvalidRecoveryId { recovery_id: u8 },
}

impl CryptoError {
    /// Cast the `CryptoError` into a `u32`, so that it can be passed across the
    /// WebAssembly FFI.
    pub fn into_error_code(self) -> u32 {
        match self {
            Self::IncorrectLength { .. } | Self::IncorrectLengths { .. } => 1,
            Self::InvalidRecoveryId { .. } => 2,
            Self::Signature(_) => 3,
        }
    }
}

pub type CryptoResult<T> = core::result::Result<T, CryptoError>;
