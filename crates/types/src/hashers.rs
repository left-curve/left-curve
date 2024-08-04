use {
    crate::{Hash160, Hash256},
    digest::Digest,
    ripemd::Ripemd160,
    sha2::Sha256,
};

pub fn hash160<T>(data: T) -> Hash160
where
    T: AsRef<[u8]>,
{
    let mut hasher = Ripemd160::new();
    hasher.update(data.as_ref());
    Hash160::from_array(hasher.finalize().into())
}

pub fn hash256<T>(data: T) -> Hash256
where
    T: AsRef<[u8]>,
{
    let mut hasher = Sha256::new();
    hasher.update(data.as_ref());
    Hash256::from_array(hasher.finalize().into())
}
