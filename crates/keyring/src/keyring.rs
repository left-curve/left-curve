use {
    crate::{prompt::{confirm, read_password, read_text}, SigningKey},
    aes_gcm::{aead::Aead, AeadCore, Aes256Gcm, Key, KeyInit},
    anyhow::ensure,
    bip32::{Language, Mnemonic},
    colored::Colorize,
    cw_std::Binary,
    pbkdf2::pbkdf2_hmac,
    rand::{rngs::OsRng, Rng},
    serde::{Deserialize, Serialize},
    sha2::Sha256,
    std::{fs, path::PathBuf},
};

// https://docs.rs/password-hash/0.5.0/password_hash/struct.Salt.html#recommended-length
const SALT_LEN: usize = 16;
// https://en.wikipedia.org/wiki/PBKDF2#:~:text=In%202023%2C%20OWASP%20recommended%20to,for%20PBKDF2%2DHMAC%2DSHA512.
#[cfg(not(debug_assertions))]
const ROUNDS: u32 = 600_000;
// in debug mode, reduce the number of rounds to make it run faster.
#[cfg(debug_assertions)]
const ROUNDS: u32 = 50_000;

/// An encrypted secp256k1 private key to be written to file.
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

    pub fn add(&self, name: &str, recover: bool, coin_type: usize) -> anyhow::Result<()> {
        // make sure that a key with the same name doesn't already exist
        let filename = self.filename(name);
        ensure!(!filename.exists(), "A signing key with name `{name}` already exists");

        // generate or recover the mnemonic phrase
        let mnemonic = if recover {
            let phrase = read_text("🔑 Enter your BIP-39 mnemonic".bold())?;
            Mnemonic::new(phrase, Language::English)?
        } else {
            Mnemonic::random(OsRng, Language::English)
        };

        // derive the signing key from mnemonic
        let sk = SigningKey::derive_from_mnemonic(&mnemonic, coin_type)?;

        // ask the user for a password
        let password = read_password(
            format!("🔑 Enter a password to encrypt key `{name}`").bold()
        )?;

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

        println!("\n{record_str}");

        if !recover {
            println!("\n{} write this mnemonic phrase in a safe place!", "Important:".bold());
            println!("It is the only way to recover your account if you ever forget your password.");
            println!("\n{}", mnemonic.phrase());
        }

        Ok(())
    }

    pub fn get(&self, name: &str) -> anyhow::Result<SigningKey> {
        let filename = self.filename(name);
        ensure!(filename.exists(), "No signing key with name `{name}` found");

        // read the file
        let record_str = fs::read_to_string(filename)?;
        let record: Record = serde_json::from_str(&record_str)?;

        // ask the user for the password
        let password = read_password(
            format!("🔑 Enter the password the was used to encrypt key `{name}`").bold()
        )?;

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

        if confirm(format!("🚨 Confirm deleting key `{name}`?").bold())? {
            fs::remove_file(filename)?;
            println!("Key `{name}` deleted");
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

        println!("{}", serde_json::to_string_pretty(&records)?);

        Ok(())
    }
}
