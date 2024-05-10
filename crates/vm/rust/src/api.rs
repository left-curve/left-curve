use {
    cw_crypto::{secp256k1_verify, secp256r1_verify},
    cw_types::{Addr, Api, StdError, StdResult},
};

pub struct ApiProvider;

impl Api for ApiProvider {
    fn debug(&self, addr: &Addr, msg: &str) {
        println!("Contract emitted debug message! addr = {addr}, msg = {msg}");
    }

    fn secp256k1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        secp256k1_verify(msg_hash, sig, pk).map_err(|_| StdError::VerificationFailed)
    }

    fn secp256r1_verify(&self, msg_hash: &[u8], sig: &[u8], pk: &[u8]) -> StdResult<()> {
        secp256r1_verify(msg_hash, sig, pk).map_err(|_| StdError::VerificationFailed)
    }
}
