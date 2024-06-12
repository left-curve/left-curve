use {
    crate::{CryptoError, CryptoResult},
    digest::{
        consts::U32, generic_array::GenericArray, FixedOutput, HashMarker, OutputSizeUser, Update,
    },
};

/// Cast a slice to a fixed length array. Error if the size is incorrect.
pub fn to_sized<const S: usize>(data: &[u8]) -> CryptoResult<[u8; S]> {
    data.try_into().map_err(|_| CryptoError::IncorrectLength {
        expect: S,
        actual: data.len(),
    })
}

#[macro_export]
macro_rules! validate_length {
    ($data:expr, $($len:expr),+) => {
        {
            let actual_length = $data.len();
            let expected_lengths = vec![$($len),+];
            if expected_lengths.contains(&actual_length) {
                Ok(())
            } else {
                Err($crate::CryptoError::IncorrectLengths {
                    expect: expected_lengths,
                    actual: actual_length,
                })
            }
        }
    };
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
    /// Convert from a byte slice of unknown length. Error if the length isn't
    /// exactly 32 bytes.
    /// To convert from a byte slice of fixed size of 32 bytes, use `from_bytes`.
    pub fn from_slice(slice: &[u8]) -> CryptoResult<Self> {
        if slice.len() != 32 {
            return Err(CryptoError::IncorrectLength {
                expect: 32,
                actual: slice.len(),
            });
        }

        Ok(Self {
            bytes: *GenericArray::from_slice(slice),
        })
    }

    pub fn into_bytes(self) -> [u8; 32] {
        self.bytes.into()
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

impl From<[u8; 32]> for Identity256 {
    fn from(bytes: [u8; 32]) -> Self {
        Self {
            bytes: *GenericArray::from_slice(&bytes),
        }
    }
}
