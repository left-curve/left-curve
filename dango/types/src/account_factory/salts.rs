use {
    crate::{
        account_factory::{AccountIndex, Username},
        auth::Key,
    },
    grug::{Binary, Hash256, Inner, JsonSerExt, SignData, StdError, StdResult},
    sha2::Sha256,
};

// ------------------------------- new user salt -------------------------------

/// Necessary data for registering a new username.
///
/// During onboarding, the account factory uses this data as salt to derive the
/// address of the user's first account. The user must make a deposit to this
/// address, then submit this data along with a signature over it.
///
/// For any subsequent account, the salt is simply the account index (in 32-bit
/// unsigned big-endian encoding).
///
/// The first ever account of a user needs a special salt, because it must
/// encode the user's username, key, and key hash, such that these cannot be
/// tempered with via frontrunning by a malicious block builder. Check the docs
/// on the user onboarding flow for more details.
#[grug::derive(Serde)]
pub struct NewUserSalt {
    pub username: Username,
    pub key: Key,
    /// An arbitrary hash used to identify the key.
    ///
    /// This is chosen by the client, without restriction on which hash algorithm
    /// to use.
    pub key_hash: Hash256,
}

impl SignData for NewUserSalt {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> StdResult<Vec<u8>> {
        // Convert to JSON value first, to ensure the structure fields are
        // sorted alphabetically.
        self.to_json_value()?.to_json_vec()
    }
}

impl NewUserSalt {
    /// Convert the salt to raw binary, as follows:
    ///
    /// ```plain
    /// bytes := len(username) || username || key_hash || key_tag || key
    /// ```
    ///
    /// The `username` has a maximum length of 15 characters. It is prefixed
    /// with a single byte indicating its length.
    ///
    /// `key_hash` doesn't need a length prefix because it's of fixed length.
    ///
    /// `key_tag` is a single byte identifying the key's type:
    ///
    /// - `0` for Secp256r1;
    /// - `1` for Secp256k1;
    /// - `2` for Ethereum address.
    pub fn to_bytes(&self) -> Vec<u8> {
        // Maximum possible length for the bytes:
        // - len(username): 1
        // - username: 15
        // - key_hash: 32
        // - key_tag: 1
        // - key: 33
        // Total: 82 bytes.
        let mut bytes = Vec::with_capacity(82);
        bytes.push(self.username.len() as u8);
        bytes.extend(self.username.inner().as_bytes());
        bytes.extend(self.key_hash.inner());
        match self.key {
            Key::Secp256r1(pk) => {
                bytes.push(0);
                bytes.extend(pk.inner());
            },
            Key::Secp256k1(pk) => {
                bytes.push(1);
                bytes.extend(pk.inner());
            },
            Key::Ethereum(addr) => {
                bytes.push(2);
                bytes.extend(addr.inner());
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
