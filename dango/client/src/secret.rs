use bip32::PublicKey;

/// Represents a secret key that can sign transactions.
pub trait Secret {}

/// An Secp256k1 private key.
pub struct Secp256k1(k256::ecdsa::SigningKey);

impl Secret for Secp256k1 {}

impl Secp256k1 {
    /// Return the private key as a byte array.
    pub fn private_key(&self) -> [u8; 32] {
        self.0.to_bytes().into()
    }

    /// Return the public key as a byte array.
    pub fn public_key(&self) -> [u8; 33] {
        self.0.verifying_key().to_bytes()
    }

    pub fn extended_public_key(&self) -> [u8; 65] {
        let a = self.0.verifying_key().to_encoded_point(false);
        a.as_bytes().try_into().unwrap()
    }
}

/// An Secp256k1 private key that signs message in Ethereum EIP-712 format.
pub struct Ethereum(k256::ecdsa::SigningKey);

impl Secret for Ethereum {}

// TODO: Secp256r1 secret.
