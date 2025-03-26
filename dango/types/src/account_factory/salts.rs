use {
    crate::{account_factory::AccountIndex, auth::Key},
    grug::{Binary, Hash256},
};

// ------------------------------- new user salt -------------------------------

/// The salt that account factory uses to create a new user's first ever account,
/// during the onboarding flow.
///
/// For any subsequent account, the salt is simply the account ID; that is, a
/// string in the format: `{username}/account/{index}`.
///
/// The first ever account of a user needs a special salt, because it must
/// encode the user's username, key, and key hash, such that these cannot be
/// tempered with via frontrunning by a malicious block builder. Check the docs
/// on the user onboarding flow for more details.
#[derive(Debug, Clone, Copy)]
pub struct NewUserSalt {
    pub secret: u32,
    pub key: Key,
    pub key_hash: Hash256,
}

impl NewUserSalt {
    /// Convert the salt to raw binary, as follows:
    ///
    /// ```plain
    /// bytes := secret (in big endian) || key_hash || key_tag || key
    /// ```
    ///
    /// `secret` is provided externally.
    ///
    /// `key_hash` doesn't need a length prefix because it's of fixed length.
    ///
    /// `key_tag` is a single byte identifying the key's type:
    /// - `0` for Secp256r1
    /// - `1` for Secp256k1
    /// - `2` for Ethereum address
    pub fn into_bytes(self) -> [u8; 70] {
        // Maximum possible length for the bytes:
        // - secret: 4
        // - key_hash: 32
        // - key_tag: 1
        // - key: 33
        // Total: 70 bytes.
        let mut bytes = [0; 70];
        bytes[0..4].copy_from_slice(&self.secret.to_be_bytes());
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
        salt.into_bytes().into()
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
