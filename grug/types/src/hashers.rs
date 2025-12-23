use {
    crate::{Hash160, Hash256},
    digest::Digest,
    ripemd::Ripemd160,
    sha2::Sha256,
    sha3::Keccak256,
};

/// Represents a data that can be hashed.
pub trait HashExt {
    /// Produce a 20-byte hash of the data using Grug's default hash algorithm.
    /// For now, we use **RIPEMD-160** as the default 20-byte hash algorithm.
    fn hash160(&self) -> Hash160 {
        self.ripemd160()
    }

    /// Produce a 32-byte hash of the data using Grug's default hash algorithm.
    /// For now, we use **SHA2-256** as the default 32-byte hash algorithm.
    fn hash256(&self) -> Hash256 {
        self.sha2_256()
    }

    /// Produce a has of the data using the RIPEMD-160 algorithm.
    fn ripemd160(&self) -> Hash160;

    /// Produce a has of the data using the SHA2-256 algorithm.
    fn sha2_256(&self) -> Hash256;

    /// Produce a has of the data using the Keccak256 algorithm.
    fn keccak256(&self) -> Hash256;
}

// Currently, we use RIPEMD-160 for 20-byte hashes, and SHA2-256 for 32-byte
// hashes. However, this can change prior to v1. We may consider BLAKE3 for `hash256`.
impl<T> HashExt for T
where
    T: AsRef<[u8]>,
{
    fn ripemd160(&self) -> Hash160 {
        let mut hasher = Ripemd160::new();
        hasher.update(self.as_ref());
        Hash160::from_inner(hasher.finalize().into())
    }

    fn sha2_256(&self) -> Hash256 {
        let mut hasher = Sha256::new();
        hasher.update(self.as_ref());
        Hash256::from_inner(hasher.finalize().into())
    }

    fn keccak256(&self) -> Hash256 {
        let mut hasher = Keccak256::new();
        hasher.update(self.as_ref());
        Hash256::from_inner(hasher.finalize().into())
    }
}
