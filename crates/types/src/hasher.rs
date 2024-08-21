use {
    crate::{Hash160, Hash256},
    digest::Digest,
    ripemd::Ripemd160,
    sha2::Sha256,
};

pub trait Hasher {
    /// Hash the data with RIPEMD160.
    fn hash160(&self) -> Hash160;

    /// Hash the data with SHA2-256.
    fn hash256(&self) -> Hash256;
}

impl<T> Hasher for T
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
