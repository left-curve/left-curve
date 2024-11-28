use {
    crate::{
        account_factory::{AccountIndex, Username},
        auth::Key,
    },
    grug::{Binary, Hash160},
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
pub struct NewUserSalt<'a> {
    pub username: &'a Username,
    pub key: Key,
    pub key_hash: Hash160,
}

impl NewUserSalt<'_> {
    /// Convert the salt to raw binary, as follows:
    ///
    /// ```plain
    /// bytes := len(username) || username || key_hash || key_tag || key
    /// ```
    ///
    /// `username` needs to be length-prefixed, because usernames can be of
    /// variable lengths, so we need to know the length to know where the
    /// username ends and where `hey_hash` starts.
    ///
    /// `key_hash` doesn't need a length prefix because it's of fixed length.
    ///
    /// `key_tag` is a single byte identifying the key's type:
    /// - `0` for Secp256r1
    /// - `1` for Secp256k1
    /// - `2` for Ed25519
    pub fn into_bytes(self) -> Vec<u8> {
        // Maximum possible length for the bytes:
        // - len(username): 1
        // - username: 15
        // - key_hash: 20
        // - key_tag: 1
        // - key: 33
        // Total: 70 bytes.
        let mut bytes = Vec::with_capacity(70);
        bytes.push(self.username.len());
        bytes.extend_from_slice(self.username.as_ref());
        bytes.extend_from_slice(&self.key_hash);
        match self.key {
            Key::Secp256r1(pk) => {
                bytes.push(0);
                bytes.extend_from_slice(&pk);
            },
            Key::Secp256k1(pk) => {
                bytes.push(1);
                bytes.extend_from_slice(&pk);
            },
        }
        bytes
    }
}

// Implement `Into<Binary>` trait, so that `NewUserSalt` can be used in the
// `Message::instantiate` method.
impl<'a> From<NewUserSalt<'a>> for Binary {
    fn from(salt: NewUserSalt<'a>) -> Self {
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
