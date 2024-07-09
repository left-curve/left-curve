use {
    crate::{CryptoError, CryptoResult},
    digest::{
        consts::{U32, U64},
        generic_array::GenericArray,
        FixedOutput, HashMarker, Output, OutputSizeUser, Update,
    },
    std::ops::Deref,
};

/// Try cast a slice to a fixed length array. Error if the size is incorrect.
pub fn to_sized<const S: usize>(data: &[u8]) -> CryptoResult<[u8; S]> {
    data.try_into().map_err(|_| CryptoError::IncorrectLength {
        expect: S,
        actual: data.len(),
    })
}

/// Truncate a slice to a fixed length array. Error if the size is less than the fixed length.
pub fn truncate<const S: usize>(data: &[u8]) -> CryptoResult<[u8; S]> {
    if data.len() < S {
        return Err(CryptoError::ExceedsMaximumLength {
            max_length: S,
            actual_length: data.len(),
        });
    }
    to_sized(&data[..S])
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
macro_rules! identity {
    ($name:ident, $array_len:ty, $len:literal, $doc:literal) => {
        #[derive(Default, Clone)]
        #[doc = $doc]
        pub struct $name {
            bytes: GenericArray<u8, $array_len>,
        }

        impl $name {
            /// Convert from a byte slice of unknown length. Error if the length isn't
            /// exactly 32 bytes.
            /// To convert from a byte slice of fixed size of 32 bytes, use `from_bytes`.
            pub fn from_slice(slice: &[u8]) -> CryptoResult<Self> {
                if slice.len() != $len {
                    return Err(CryptoError::IncorrectLength {
                        expect: $len,
                        actual: slice.len(),
                    });
                }

                Ok(Self {
                    bytes: *GenericArray::from_slice(slice),
                })
            }

            pub fn into_bytes(self) -> [u8; $len] {
                self.bytes.into()
            }

            pub fn as_bytes(&self) -> &[u8] {
                &self.bytes
            }
        }

        impl From<[u8; $len]> for $name {
            fn from(bytes: [u8; $len]) -> Self {
                Self {
                    bytes: *GenericArray::from_slice(&bytes),
                }
            }
        }

        impl Deref for $name {
            type Target = [u8];

            fn deref(&self) -> &Self::Target {
                &self.bytes
            }
        }

        impl Update for $name {
            fn update(&mut self, data: &[u8]) {
                assert_eq!(data.len(), $len);
                self.bytes = *GenericArray::from_slice(data);
            }
        }

        impl FixedOutput for $name {
            fn finalize_into(self, out: &mut Output<Self>) {
                *out = self.bytes;
            }
        }

        impl OutputSizeUser for $name {
            type OutputSize = $array_len;
        }

        impl HashMarker for $name {}
    };
}

identity!(Identity256, U32, 32, "A digest of 32 byte length");
identity!(Identity512, U64, 64, "A digest of 64 byte length");
