use {
    crate::{account_factory::AccountIndex, auth::Key},
    grug::{Binary, Hash256},
};

// ------------------------------- new user salt -------------------------------

/// Salt used by the account factory to derive the deposit address for a user
/// user who is to be unobarded. The user must make a deposit to this address.
///
/// For any subsequent account, the salt is simply the account index (in 32-bit
/// unsigned big-endian encoding).
///
/// The first ever account of a user needs a special salt, because it must
/// encode the user's key and key hash, such that these cannot be tempered with
/// via frontrunning by a malicious block builder. Check the docs on the user
/// onboarding flow for more details.
#[grug::derive(Serde)]
pub struct NewUserSalt {
    pub key: Key,
    /// An arbitrary hash used to identify the key.
    ///
    /// This is chosen by the client, without restriction on which hash algorithm
    /// to use.
    pub key_hash: Hash256,
    /// An arbitrary number chosen by the user, to give more variety to the
    /// derived deposit address.
    pub seed: u32,
}

impl NewUserSalt {
    /// Convert the salt to raw binary, as follows:
    ///
    /// ```plain
    /// bytes := seed (in big endian) || key_hash || key_tag || key
    /// ```
    ///
    /// - `seed` is a 4-byte integer, in big-endian encoding.
    /// - `key_hash` doesn't need a length prefix because it's of fixed length.
    /// - `key_tag` is a single byte identifying the key's type:
    ///   - `0` for Secp256r1;
    ///   - `1` for Secp256k1;
    ///   - `2` for Ethereum address.
    pub fn to_bytes(&self) -> [u8; 70] {
        // Maximum possible length for the bytes:
        // - seed: 4
        // - key_hash: 32
        // - key_tag: 1
        // - key: 33
        // Total: 70 bytes.
        let mut bytes = [0; 70];
        bytes[0..4].copy_from_slice(&self.seed.to_be_bytes());
        bytes[4..36].copy_from_slice(&self.key_hash);
        match self.key {
            Key::Secp256r1(pk) => {
                bytes[36] = 0;
                bytes[37..70].copy_from_slice(&pk);
            },
            Key::Secp256k1(pk) => {
                bytes[36] = 1;
                bytes[37..70].copy_from_slice(&pk);
            },
            Key::Ethereum(addr) => {
                bytes[36] = 2;
                // Front-pad the address with zeros.
                bytes[50..70].copy_from_slice(&addr);
            },
        }
        bytes
    }
}

// Implement `Into<Binary>` trait, so that `NewUserSalt` can be used in the
// `Message::instantiate` method.
impl From<NewUserSalt> for Binary {
    fn from(salt: NewUserSalt) -> Self {
        salt.to_bytes().into()
    }
}

// ------------------------------- regular salt --------------------------------

/// The salt account factory uses to create a user's subsequent accounts.
#[derive(Debug, Clone, Copy)]
pub struct Salt {
    pub index: AccountIndex,
}

impl Salt {
    pub fn into_bytes(self) -> Vec<u8> {
        self.index.to_be_bytes().to_vec()
    }
}

impl From<Salt> for Binary {
    fn from(salt: Salt) -> Self {
        salt.into_bytes().into()
    }
}
