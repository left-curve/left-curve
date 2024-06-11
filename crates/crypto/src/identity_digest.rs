use {
    crate::{CryptoError, CryptoResult},
    digest::{
        consts::U32, generic_array::GenericArray, FixedOutput, HashMarker, OutputSizeUser, Update,
    },
};

// a convenience method for use in tests.
#[cfg(test)]
pub(crate) fn hash(data: &[u8]) -> Identity256 {
    use digest::Digest;

    let mut hasher = sha2::Sha256::new();
    // I don't fully understand why it's necessary to use this syntax instead of
    // hasher.update(data)

    Digest::update(&mut hasher, data);
    let bytes = hasher.finalize();

    Identity256 { bytes }
}

/// To utilize the `signature::DigestVerifier::verify_digest` method, the digest
/// must implement the `digest::Digest` trait, which in turn requires the
/// following traits:
///
/// - Default
/// - OutputSizeUser
/// - Update
/// - FixedOutput
/// - HashMarker
///
/// Here we define a container struct that implements the required traits.
///
/// Adapted from:
/// <https://github.com/CosmWasm/cosmwasm/blob/main/packages/crypto/src/identity_digest.rs>
#[derive(Default, Clone)]
pub struct Identity256 {
    bytes: GenericArray<u8, U32>,
}

impl Identity256 {
    /// Convert from a byte slice of fixed length of 32 bytes.
    /// To convert from byte slices of unknown lengths, use `from_slice`.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            bytes: *GenericArray::from_slice(bytes),
        }
    }

    /// Convert from a byte slice of unknown length. Error if the length isn't
    /// exactly 32 bytes.
    /// To convert from a byte slice of fixed size of 32 bytes, use `from_bytes`.
    pub fn from_slice(slice: &[u8]) -> CryptoResult<Self> {
        if slice.len() != 32 {
            return Err(CryptoError::incorrect_length(32, slice.len()));
        }

        Ok(Self {
            bytes: *GenericArray::from_slice(slice),
        })
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl OutputSizeUser for Identity256 {
    type OutputSize = U32;
}

impl Update for Identity256 {
    fn update(&mut self, data: &[u8]) {
        assert_eq!(data.len(), 32);
        self.bytes = *GenericArray::from_slice(data);
    }
}

impl FixedOutput for Identity256 {
    fn finalize_into(self, out: &mut digest::Output<Self>) {
        *out = self.bytes
    }
}

impl HashMarker for Identity256 {}
