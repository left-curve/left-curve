use {
    aes_gcm::{aead::Aead, AeadCore, Aes256Gcm, Key, KeyInit},
    bip32::{Mnemonic, PublicKey, XPrv},
    grug::{Binary, ByteArray, JsonDeExt, JsonSerExt},
    grug_crypto::Identity256,
    k256::ecdsa::{signature::DigestSigner, Signature},
    pbkdf2::pbkdf2_hmac,
    rand::{rngs::OsRng, Rng},
    sha2::Sha256,
    std::{fs, path::Path},
};

const SECP256K1_COMPRESSED_PUBKEY_LEN: usize = 33;
const PBKDF2_ITERATIONS: u32 = 600_000;
const PBKDF2_SALT_LEN: usize = 16;
const PBKDF2_KEY_LEN: usize = 32;
const AES256GCM_NONCE_LEN: usize = 12;

/// [`SigningKey`](crate::SigningKey) serialized into JSON format, to be stored
/// on disk.
#[grug::derive(Serde)]
pub struct Keystore {
    pub pk: ByteArray<SECP256K1_COMPRESSED_PUBKEY_LEN>,
    pub salt: ByteArray<PBKDF2_SALT_LEN>,
    pub nonce: ByteArray<AES256GCM_NONCE_LEN>,
    pub ciphertext: Binary,
}

/// A wrapper over an Secp256k1 [`SigningKey`](k256::ecdsa::SigningKey),
/// providing a handy API to work with.
#[derive(Debug, Clone)]
pub struct SigningKey {
    inner: k256::ecdsa::SigningKey,
}

impl SigningKey {
    /// Generate a random Secp256k1 private key.
    pub fn new_random() -> Self {
        Self {
            inner: k256::ecdsa::SigningKey::random(&mut OsRng),
        }
    }

    /// Recover an Secp256k1 private key from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> anyhow::Result<Self> {
        Ok(Self {
            inner: k256::ecdsa::SigningKey::from_bytes(&bytes.into())?,
        })
    }

    /// Recover an Secp256k1 private key from the given English mnemonic and
    /// BIP-44 coin type.
    pub fn from_mnemonic(mnemonic: &Mnemonic, coin_type: usize) -> anyhow::Result<Self> {
        // The `to_seed` function takes a password to generate salt.
        // Here we just use an empty str.
        // For reference, Terra Station and Keplr use an empty string as well:
        // - https://github.com/terra-money/terra.js/blob/v3.1.7/src/key/MnemonicKey.ts#L79
        // - https://github.com/chainapsis/keplr-wallet/blob/b6062a4d24f3dcb15dda063b1ece7d1fbffdbfc8/packages/crypto/src/mnemonic.ts#L63
        let seed = mnemonic.to_seed("");
        let path = format!("m/44'/{coin_type}'/0'/0/0");
        let xprv = XPrv::derive_from_path(&seed, &path.parse()?)?;

        Ok(Self { inner: xprv.into() })
    }

    /// Read and decrypt a keystore file.
    pub fn from_file<F, P>(filename: F, password: P) -> anyhow::Result<Self>
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
        let decrypted =
            cipher.decrypt(keystore.nonce.as_ref().into(), keystore.ciphertext.as_ref())?;

        Ok(Self {
            inner: k256::ecdsa::SigningKey::from_bytes(decrypted.as_slice().into())?,
        })
    }

    /// Encrypt a key and save it to a file.
    pub fn write_to_file<F, P>(&self, filename: F, password: P) -> anyhow::Result<Keystore>
    where
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
        let ciphertext = cipher.encrypt(&nonce, self.private_key().as_slice())?;

        // write keystore to file
        let keystore = Keystore {
            pk: self.public_key().into(),
            salt: salt.into(),
            nonce: nonce.as_slice().try_into()?,
            ciphertext: ciphertext.into(),
        };
        let keystore_str = keystore.to_json_string_pretty()?;
        fs::write(filename, keystore_str.as_bytes())?;

        Ok(keystore)
    }

    /// Sign the given digest.
    pub fn sign_digest(&self, digest: [u8; 32]) -> [u8; 64] {
        let digest = Identity256::from(digest);
        let signature: Signature = self.inner.sign_digest(digest);
        signature.to_bytes().into()
    }

    /// Return the private key as a byte array.
    pub fn private_key(&self) -> [u8; 32] {
        self.inner.to_bytes().into()
    }

    /// Return the public key as a byte array.
    pub fn public_key(&self) -> [u8; 33] {
        self.inner.verifying_key().to_bytes()
    }
}
