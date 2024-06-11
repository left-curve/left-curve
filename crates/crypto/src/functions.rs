use crate::{CryptoError, CryptoResult};

pub(crate) fn assert_len(data: &[u8], len: usize) -> CryptoResult<()> {
    if data.len() == len {
        Ok(())
    } else {
        Err(CryptoError::incorrect_length(32, data.len()))
    }
}
