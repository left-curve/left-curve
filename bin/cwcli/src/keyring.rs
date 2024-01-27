use {
    crate::prompt::{confirm, read_password},
    aes_gcm::{aead::Aead, AeadCore, Aes256Gcm, Key, KeyInit},
    anyhow::ensure,
    bip32::{Mnemonic, XPrv},
    colored::Colorize,
    cw_crypto::Identity256,
    cw_std::{Addr, Binary, Message, Tx},
    k256::ecdsa::{signature::DigestSigner, Signature, VerifyingKey},
    pbkdf2::pbkdf2_hmac,
    rand::{rngs::OsRng, Rng},
    serde::{Deserialize, Serialize},
    sha2::Sha256,
    std::{fs, path::PathBuf},
};

// -------------------------------- SigningKey ---------------------------------

/// A wrapper over k256 SigningKey, providing a handy API to work with.
pub struct SigningKey {
    inner: k256::ecdsa::SigningKey,
}

impl SigningKey {
    /// Note: Only support secp256k1, not r1. This is because we use Bitcoin's
    /// BIP-32 library, and Bitcoin only uses k1.
    pub fn derive_from_mnemonic(mnemonic: &Mnemonic, coin_type: usize) -> anyhow::Result<Self> {
        // The `to_seed` function takes a password to generate salt.
        // Here we just use an empty str.
        // For reference, Terra Station and Keplr use an empty string as well:
        // - https://github.com/terra-money/terra.js/blob/v3.1.7/src/key/MnemonicKey.ts#L79
        // - https://github.com/chainapsis/keplr-wallet/blob/b6062a4d24f3dcb15dda063b1ece7d1fbffdbfc8/packages/crypto/src/mnemonic.ts#L63
        let seed = mnemonic.to_seed("");
        let path = format!("m/44'/{coin_type}'/0'/0/0");
        let xprv = XPrv::derive_from_path(&seed, &path.parse()?)?;
        Ok(Self {
            inner: xprv.into(),
        })
    }

    /// Note: The transaction is signed using the method defined by the default
    /// cw-account contract. This CLI tool is intended to work with this account
    /// type only. Projects that wish to create their custom account types may
    /// consider creating their custom CLI tools or scripting libraries.
    pub fn create_and_sign_tx(
        &self,
        sender: Addr,
        msgs: Vec<Message>,
        chain_id: &str,
        sequence: u32,
    ) -> anyhow::Result<Tx> {
        let sign_bytes = cw_account::sign_bytes(&msgs, &sender, chain_id, sequence)?;
        let sign_bytes = Identity256::from_bytes(&sign_bytes);
        let signature: Signature = self.inner.sign_digest(sign_bytes);
        Ok(Tx {
            sender,
            msgs,
            credential: signature.to_vec().into(),
        })
    }

    fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes().into()
    }

    fn verifying_key(&self) -> &VerifyingKey {
        self.inner.verifying_key()
    }
}

// ---------------------------------- Keyring ----------------------------------

// https://docs.rs/password-hash/0.5.0/password_hash/struct.Salt.html#recommended-length
const SALT_LEN: usize = 16;
// https://en.wikipedia.org/wiki/PBKDF2#:~:text=In%202023%2C%20OWASP%20recommended%20to,for%20PBKDF2%2DHMAC%2DSHA512.
const ROUNDS: u32 = 600_000;

#[derive(Serialize, Deserialize)]
pub struct Record {
    name:       String,
    pubkey:     Binary,
    salt:       Binary,
    nonce:      Binary,
    ciphertext: Binary,
}

/// This is similar to Cosmos SDK's file keyring. All keys are encrypted using
/// the same password and saved to disk.
pub struct Keyring {
    dir: PathBuf,
}

impl Keyring {
    pub fn open(dir: PathBuf) -> anyhow::Result<Self> {
        // create the directory if not exist
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }

        Ok(Self {
            dir,
        })
    }

    pub fn filename(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.json"))
    }

    pub fn add(&self, name: &str, sk: &SigningKey) -> anyhow::Result<()> {
        let filename = self.filename(name);
        ensure!(!filename.exists(), "A signing key with name `{name}` already exists");

        // ask the user for a password
        let password = read_password("🔑 Enter a password to encrypt the key".bold())?;

        // generate a random salt for use in encryption key derivation
        let mut salt = [0u8; SALT_LEN];
        OsRng.fill(&mut salt);

        // derive the encryption key using PBKDF2-HMAC
        let mut password_hash = [0u8; 32];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, ROUNDS, &mut password_hash);

        // encrypt the signing key
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&password_hash));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, sk.to_bytes().as_slice())?;

        // save the record to file
        let record = Record {
            name:       name.into(),
            pubkey:     sk.verifying_key().to_sec1_bytes().to_vec().into(),
            salt:       salt.to_vec().into(),
            nonce:      nonce.to_vec().into(),
            ciphertext: ciphertext.into(),
        };
        let record_str = serde_json::to_string_pretty(&record)?;
        fs::write(&filename, record_str.as_bytes())?;

        println!("{record_str}");

        Ok(())
    }

    pub fn get(&self, name: &str) -> anyhow::Result<SigningKey> {
        let filename = self.filename(name);
        ensure!(filename.exists(), "No signing key with name `{name}` found");

        // read the file
        let record_str = fs::read_to_string(filename)?;
        let record: Record = serde_json::from_str(&record_str)?;

        // ask the user for the password
        let password = read_password("🔑 Enter the password the was used to encrypt the key".bold())?;

        // derive the password, using the saved salt
        let mut password_hash = [0u8; 32];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), &record.salt, ROUNDS, &mut password_hash);

        // descrypt the signing key
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&password_hash));
        let sk_bytes = cipher.decrypt(record.nonce.as_ref().into(), record.ciphertext.as_ref())?;
        let sk = k256::ecdsa::SigningKey::from_bytes(sk_bytes.as_slice().into())?;

        Ok(SigningKey { inner: sk })
    }

    pub fn delete(&self, name: &str) -> anyhow::Result<()> {
        let filename = self.filename(name);
        ensure!(filename.exists(), "No signing key with name `{name}` found");

        if confirm("Confirm deleting key `{name}`?")? {
            fs::remove_file(filename)?;
        }

        Ok(())
    }

    pub fn show(&self, name: &str) -> anyhow::Result<()> {
        let filename = self.filename(name);
        ensure!(filename.exists(), "No signing key with name `{name}` found");

        let record_str = fs::read_to_string(filename)?;
        println!("{record_str}");

        Ok(())
    }

    pub fn list(&self) -> anyhow::Result<()> {
        let mut records = vec![];
        for entry in self.dir.read_dir()? {
            let entry = entry?;
            let record_str = fs::read_to_string(entry.path())?;
            let record: Record = serde_json::from_str(&record_str)?;
            records.push(record);
        }

        let records_str = serde_json::to_string_pretty(&records)?;
        println!("{records_str}");

        Ok(())
    }
}
