//! Utilities related to Ethereum.

use {
    grug::{Hash256, HashExt, Inner},
    identity::Identity256,
};

/// An Ethereum address.
pub type Address = [u8; 20];

/// An Secp256k1 private key.
pub type SigningKey = [u8; 32];

/// An Secp256k1 public key in compressed form.
pub type VerifyingKey = [u8; 33];

/// An Secp256k1 signature packed into the format expected by Ethereum.
pub type Signature = [u8; 65];

/// Derive an Ethereum address from a Secp256k1 public key.
pub fn derive_address(vk: &k256::ecdsa::VerifyingKey) -> Address {
    let vk_raw = vk.to_encoded_point(false);
    let vk_hash = (&vk_raw.as_bytes()[1..]).keccak256();
    let address = &vk_hash[12..];
    address.try_into().unwrap()
}

/// Sign the given _hashed_ message with the given private key, and pack the
/// signature into the format expected by Ethereum.
pub fn sign(msg_hash: Hash256, sk: &k256::ecdsa::SigningKey) -> Signature {
    let (signature, recovery_id) = sk
        .sign_digest_recoverable(Identity256::from(msg_hash.into_inner()))
        .unwrap();
    pack_signature(signature, recovery_id)
}

/// Convert a recoverable Secp256k1 signature produced by the k256 library
/// into a 65-byte signature expected by Ethereum.
pub fn pack_signature(
    signature: k256::ecdsa::Signature,
    recovery_id: k256::ecdsa::RecoveryId,
) -> Signature {
    let mut packed = [0u8; 65];
    packed[..64].copy_from_slice(&signature.to_bytes());
    packed[64] = recovery_id.to_byte() + 27;
    packed
}
