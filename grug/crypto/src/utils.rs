use crate::{CryptoError, CryptoResult};

/// Try cast a slice to a fixed length array. Error if the size is incorrect.
pub fn to_sized<const S: usize>(data: &[u8]) -> CryptoResult<[u8; S]> {
    data.try_into().map_err(|_| CryptoError::IncorrectLength {
        expect: S,
        actual: data.len(),
    })
}

/// Truncate a slice to a fixed length array.
/// Panic if the size is less than the fixed length.
pub fn truncate<const S: usize>(data: &[u8]) -> [u8; S] {
    debug_assert!(
        data.len() >= S,
        "can't truncate a slice of length {} to a longer length {}",
        data.len(),
        S
    );

    let mut out = [0_u8; S];
    if data.len() >= S {
        out.copy_from_slice(&data[..S]);
    } else {
        out[..data.len()].copy_from_slice(data);
    }
    out
}
