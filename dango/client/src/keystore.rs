use {
    crate::Secret,
    aes_gcm::{AeadCore, Aes256Gcm, Key, KeyInit, aead::Aead},
    anyhow::anyhow,
    grug::{Binary, ByteArray, JsonDeExt, JsonSerExt},
    pbkdf2::pbkdf2_hmac,
    rand::{Rng, rngs::OsRng},
    sha2::Sha256,
    std::{fs, path::Path},
};

const SECP256K1_COMPRESSED_PUBKEY_LEN: usize = 33;
const PBKDF2_ITERATIONS: u32 = 600_000;
const PBKDF2_SALT_LEN: usize = 16;
const PBKDF2_KEY_LEN: usize = 32;
const AES256GCM_NONCE_LEN: usize = 12;

/// Data structure for encrypting a 32-byte private key before saving on disk.
#[grug::derive(Serde)]
pub struct Keystore {
    pub pk: ByteArray<SECP256K1_COMPRESSED_PUBKEY_LEN>,
    pub salt: ByteArray<PBKDF2_SALT_LEN>,
    pub nonce: ByteArray<AES256GCM_NONCE_LEN>,
    pub ciphertext: Binary,
}

impl Keystore {
    /// Read and decrypt a keystore file.
    pub fn from_file<F, P>(filename: F, password: P) -> anyhow::Result<[u8; 32]>
    where
        F: AsRef<Path>,
        P: AsRef<[u8]>,
    {
        // read keystore file
        let keystore_str = fs::read_to_string(filename)?;
        let keystore: Keystore = keystore_str.deserialize_json()?;

        // recover encryption key from password and salt
        let mut password_hash = [0u8; PBKDF2_KEY_LEN];
        pbkdf2_hmac::<Sha256>(
            password.as_ref(),
            &keystore.salt,
            PBKDF2_ITERATIONS,
            &mut password_hash,
        );

        // decrypt the private key
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&password_hash));

        cipher
            .decrypt(keystore.nonce.as_ref().into(), keystore.ciphertext.as_ref())?
            .try_into()
            .map_err(|bytes: Vec<u8>| {
                anyhow!(
                    "incorrect private key length! expecting: 32, got: {}",
                    bytes.len()
                )
            })
    }

    /// Encrypt a key and save it to a file.
    pub fn write_to_file<S, F, P>(secret: &S, filename: F, password: P) -> anyhow::Result<Self>
    where
        S: Secret,
        S::Private: AsRef<[u8]>,
        S::Public: Into<ByteArray<SECP256K1_COMPRESSED_PUBKEY_LEN>>,
        F: AsRef<Path>,
        P: AsRef<[u8]>,
    {
        // generate encryption key
        let mut salt = [0u8; PBKDF2_SALT_LEN];
        OsRng.fill(&mut salt);
        let mut password_hash = [0u8; PBKDF2_KEY_LEN];
        pbkdf2_hmac::<Sha256>(
            password.as_ref(),
            &salt,
            PBKDF2_ITERATIONS,
            &mut password_hash,
        );

        // encrypt the private key
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&password_hash));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, secret.private_key().as_ref())?;

        // write keystore to file
        let keystore = Keystore {
            pk: secret.public_key().into(),
            salt: salt.into(),
            nonce: nonce.as_slice().try_into()?,
            ciphertext: ciphertext.into(),
        };
        let keystore_str = keystore.to_json_string_pretty()?;
        fs::write(filename, keystore_str.as_bytes())?;

        Ok(keystore)
    }
}
