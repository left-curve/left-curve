// For benchmarks purposes, crates crypto is not used
// because it would require changes in order to be used inside a contract
// (is not possible to build in wasm32-unknow-unknown target without removing some features).
// The following functions have been copied from grug-crypto.

use {
    digest::{
        consts::U32, generic_array::GenericArray, Digest, FixedOutput, HashMarker, OutputSizeUser,
        Update,
    },
    grug::{StdError, StdResult},
    k256::ecdsa::{signature::DigestVerifier, Signature, VerifyingKey},
    sha2::Sha256,
};

pub(crate) fn secp256k1_verify(msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
    let msg = Identity256::from_slice(msg_hash)?;

    let mut sig = Signature::from_bytes(sig.into()).unwrap();

    if let Some(normalized) = sig.normalize_s() {
        sig = normalized;
    }

    VerifyingKey::from_sec1_bytes(pk)
        .unwrap()
        .verify_digest(msg, &sig)
        .map_err(|_| StdError::generic_err("secp256k1_verify failed"))
}

pub(crate) fn sha2_256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

pub(crate) fn blake3(data: &[u8]) -> [u8; 32] {
    blake3::hash(data).into()
}

#[derive(Default, Clone)]
struct Identity256 {
    bytes: GenericArray<u8, U32>,
}

impl Identity256 {
    pub fn from_slice(slice: &[u8]) -> StdResult<Self> {
        if slice.len() != 32 {
            return StdResult::Err(StdError::generic_err("from_slice"));
        }

        Ok(Self {
            bytes: *GenericArray::from_slice(slice),
        })
    }
}

impl From<[u8; 32]> for Identity256 {
    fn from(bytes: [u8; 32]) -> Self {
        Self {
            bytes: *GenericArray::from_slice(&bytes),
        }
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
