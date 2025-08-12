use {
    digest::{
        FixedOutput, HashMarker, Output, OutputSizeUser, Update,
        consts::{U32, U64},
        generic_array::GenericArray,
    },
    std::ops::Deref,
};

/// To use various methods in the RustCrypto project, such as:
///
/// - `signature::DigestVerifier::verify_digest` and
/// - `ecdsa::SigningKey::sign_digest_recoverable`,
///
/// the input digest must implement the `digest::Digest` trait. However, this
/// trait isn't implemented for fixed-size arrays such as `[u8; 32]` or `[u8; 64]`.
/// This makes the methods extra tricky to use.
///
/// Here we define a wrapper struct that implements the required traits.
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
            pub const fn from_inner(bytes: GenericArray<u8, $array_len>) -> Self {
                Self { bytes }
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
