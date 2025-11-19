use crate::{CryptoError, CryptoResult};

/// Try cast a slice to a fixed length array. Error if the size is incorrect.
pub fn to_sized<const S: usize>(data: &[u8]) -> CryptoResult<[u8; S]> {
    data.try_into().map_err(|_| CryptoError::IncorrectLength {
        expect: S,
        actual: data.len(),
    })
}

/// Truncate a slice to a fixed length array.
///
/// ## Panics
///
/// Panic if the input slice's length is smaller than the output length.
pub fn truncate<const S: usize>(data: &[u8]) -> [u8; S] {
    debug_assert!(
        data.len() >= S,
        "can't truncate a slice of length {} to a longer length {}",
        data.len(),
        S
    );

    #[allow(
        clippy::unwrap_used,
        reason = "we have ensured the slice has a length equal or greater than S; its index range 0..S necessarily has length S"
    )]
    data[..S].try_into().unwrap()
}
