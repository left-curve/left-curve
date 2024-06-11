use crate::{CryptoError, CryptoResult};

pub(crate) fn to_sized<const S: usize>(data: &[u8]) -> CryptoResult<[u8; S]> {
    data.try_into()
        .map_err(|_| CryptoError::incorrect_length(S, data.len()))
}
