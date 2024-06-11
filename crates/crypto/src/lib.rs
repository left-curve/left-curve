mod ed25519;
mod error;
mod functions;
mod identity_digest;
mod secp256k1;
mod secp256r1;

pub use crate::{
    ed25519::{ed25519_batch_verify, ed25519_verify},
    error::{CryptoError, CryptoResult},
    identity_digest::Identity256,
    secp256k1::{secp256k1_pubkey_recover, secp256k1_verify},
    secp256r1::secp256r1_verify,
};
