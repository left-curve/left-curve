use grug_types::{Addr, Api, StdResult, VerificationError};

// This is named `InternalApi` to contrast with `grug_ffi::ExternalApi`, which
// works across the FFI boundary, which this doesn't.
pub struct InternalApi;

impl Api for InternalApi {
    fn debug(&self, addr: Addr, msg: &str) {
        println!("Contract emitted debug message! addr = {addr}, msg = {msg}");
    }

    fn secp256r1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        grug_crypto::secp256r1_verify(msg_hash, sig, pk)
            .map_err(|err| VerificationError::from_error_code(err.into_error_code()).into())
    }

    fn secp256k1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        grug_crypto::secp256k1_verify(msg_hash, sig, pk)
            .map_err(|err| VerificationError::from_error_code(err.into_error_code()).into())
    }

    fn secp256k1_pubkey_recover(
        &self,
        msg_hash: &[u8],
        sig: &[u8],
        recovery_id: u8,
        compressed: bool,
    ) -> StdResult<Vec<u8>> {
        grug_crypto::secp256k1_pubkey_recover(msg_hash, sig, recovery_id, compressed)
            .map_err(|err| VerificationError::from_error_code(err.into_error_code()).into())
    }

    fn ed25519_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        grug_crypto::ed25519_verify(msg_hash, sig, pk)
            .map_err(|err| VerificationError::from_error_code(err.into_error_code()).into())
    }

    fn ed25519_batch_verify(
        &self,
        prehash_msgs: &[&[u8]],
        sigs: &[&[u8]],
        pks: &[&[u8]],
    ) -> StdResult<()> {
        grug_crypto::ed25519_batch_verify(prehash_msgs, sigs, pks)
            .map_err(|err| VerificationError::from_error_code(err.into_error_code()).into())
    }

    fn sha2_256(&self, data: &[u8]) -> [u8; 32] {
        grug_crypto::sha2_256(data)
    }

    fn sha2_512(&self, data: &[u8]) -> [u8; 64] {
        grug_crypto::sha2_512(data)
    }

    fn sha2_512_truncated(&self, data: &[u8]) -> [u8; 32] {
        grug_crypto::sha2_512_truncated(data)
    }

    fn sha3_256(&self, data: &[u8]) -> [u8; 32] {
        grug_crypto::sha3_256(data)
    }

    fn sha3_512(&self, data: &[u8]) -> [u8; 64] {
        grug_crypto::sha3_512(data)
    }

    fn sha3_512_truncated(&self, data: &[u8]) -> [u8; 32] {
        grug_crypto::sha3_512_truncated(data)
    }

    fn keccak256(&self, data: &[u8]) -> [u8; 32] {
        grug_crypto::keccak256(data)
    }

    fn blake2s_256(&self, data: &[u8]) -> [u8; 32] {
        grug_crypto::blake2s_256(data)
    }

    fn blake2b_512(&self, data: &[u8]) -> [u8; 64] {
        grug_crypto::blake2b_512(data)
    }

    fn blake3(&self, data: &[u8]) -> [u8; 32] {
        grug_crypto::blake3(data)
    }
}
