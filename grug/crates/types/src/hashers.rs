use {
    crate::{Hash160, Hash256},
    digest::Digest,
    ripemd::Ripemd160,
    sha2::Sha256,
};

/// Represents a data that can be hashed.
pub trait HashExt {
    /// Hash the data, producing a 20-byte hash.
    fn hash160(&self) -> Hash160;

    /// Hash the data, producing a 32-byte hash.
    fn hash256(&self) -> Hash256;
}

// Currently, we use RIPEMD-160 for 20-byte hashes, and SHA2-256 for 32-byte
// hashes. However, this can change prior to v1. We may consider BLAKE3 for `hash256`.
impl<T> HashExt for T
where
    T: AsRef<[u8]>,
{
    fn hash160(&self) -> Hash160 {
        let mut hasher = Ripemd160::new();
        hasher.update(self.as_ref());
        Hash160::from_array(hasher.finalize().into())
    }

    fn hash256(&self) -> Hash256 {
        let mut hasher = Sha256::new();
        hasher.update(self.as_ref());
        Hash256::from_array(hasher.finalize().into())
    }
}
