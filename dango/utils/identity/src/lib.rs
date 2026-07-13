//! To use various methods in the RustCrypto project, such as:
//!
//! - `signature::DigestVerifier::verify_digest` and
//! - `ecdsa::SigningKey::sign_digest_recoverable`,
//!
//! the input digest must implement the `digest::Digest` trait. However, this
//! trait isn't implemented for fixed-size arrays such as `[u8; 32]` or `[u8; 64]`.
//! This makes the methods extra tricky to use.
//!
//! Here we define wrapper structs that implement the required traits.
//!
//! Adapted from:
//! [cosmwasm-crypto](https://github.com/CosmWasm/cosmwasm/blob/main/packages/crypto/src/identity_digest.rs)

// TODO: `generic-array` 0.14.9 deprecated the entire `GenericArray` type to push
// users toward 1.x, but `digest 0.10` (latest stable) still re-exports 0.14.
// Suppress until the RustCrypto ecosystem ships stable `digest 0.11`.
// Relevant RustCrypto crates: sha2, ecdsa, k256, p256, hmac, signature, curve25519-dalek
#![allow(deprecated)]

use {
    digest::{
        FixedOutput, HashMarker, Output, OutputSizeUser, Update, consts::U32,
        generic_array::GenericArray,
    },
    std::ops::Deref,
};

/// A digest of 32 byte length.
#[derive(Default, Clone)]
pub struct Identity256 {
    bytes: [u8; 32],
}

impl Identity256 {
    pub fn from_inner(bytes: impl Into<[u8; 32]>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }

    pub fn into_bytes(self) -> [u8; 32] {
        self.bytes
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<[u8; 32]> for Identity256 {
    fn from(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }
}

impl Deref for Identity256 {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl Update for Identity256 {
    fn update(&mut self, data: &[u8]) {
        self.bytes = data.try_into().expect("data length mismatch");
    }
}

impl FixedOutput for Identity256 {
    fn finalize_into(self, out: &mut Output<Self>) {
        *out = GenericArray::from(self.bytes);
    }
}

impl OutputSizeUser for Identity256 {
    type OutputSize = U32;
}

impl HashMarker for Identity256 {}
