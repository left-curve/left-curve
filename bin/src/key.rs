use {
    crate::prompt::{confirm, print_json_pretty, read_password, read_text},
    anyhow::ensure,
    bip32::{Language, Mnemonic},
    clap::Parser,
    colored::Colorize,
    grug_sdk::{Keystore, SigningKey},
    grug_types::from_json_slice,
    rand::rngs::OsRng,
    std::{
        fs,
        path::{Path, PathBuf},
    },
};

/// We we the the BIP-44 coin type as Ethereum for better compatibility:
/// https://github.com/satoshilabs/slips/blob/master/slip-0044.md
const DEFAULT_COIN_TYPE: usize = 60;

#[derive(Parser)]
pub enum KeyCmd {
    /// Create a new or recover an existing secp256k1 private key an save it to
    /// an encrypted file.
    Add {
        /// A human-readable name for the key
        name: String,
        /// Recover an existing seed phrase instead of generating a new one
        #[arg(long)]
        recover: bool,
        /// BIP-44 coin type for key derivation
        #[arg(long, default_value_t = DEFAULT_COIN_TYPE)]
        coin_type: usize,
    },
    /// Delete a key by name
    #[command(alias = "rm")]
    Delete {
        /// Name of the key to delete
        name: String,
    },
    /// Display details of a key by name
    Show {
        /// Name of the key to display
        name: String,
    },
    /// List all keys
    #[command(alias = "ls")]
    List,
}

impl KeyCmd {
    pub fn run(self, dir: PathBuf) -> anyhow::Result<()> {
        match self {
            KeyCmd::Add {
                name,
                recover,
                coin_type,
            } => add(&dir, &name, recover, coin_type),
            KeyCmd::Delete {
                name,
            } => delete(&dir, &name),
            KeyCmd::Show {
                name,
            } => show(&dir, &name),
            KeyCmd::List => list(&dir),
        }
    }
}

fn add(dir: &Path, name: &str, recover: bool, coin_type: usize) -> anyhow::Result<()> {
    let filename = dir.join(format!("{name}.json"));
    ensure!(!filename.exists(), "file `{filename:?}` already exists");

    // generate or recover mnemonic phrase
    let mnemonic = if recover {
        let phrase = read_text("🔑 Enter your BIP-39 mnemonic".bold())?;
        Mnemonic::new(phrase, Language::English)?
    } else {
        Mnemonic::random(OsRng, Language::English)
    };

    // ask for password and save encrypted keystore
    let password = read_password(format!("🔑 Enter a password to encrypt file `{filename:?}`").bold())?;
    let sk = SigningKey::from_mnemonic(&mnemonic, coin_type)?;
    let keystore = sk.write_to_file(&filename, &password)?;

    println!();
    print_json_pretty(keystore)?;

    if !recover {
        println!("\n{} write this mnemonic phrase in a safe place!", "Important:".bold());
        println!("It is the only way to recover your account if you ever forget your password.");
        println!("\n{}", mnemonic.phrase());
    }

    Ok(())
}

fn delete(dir: &Path, name: &str) -> anyhow::Result<()> {
    let filename = dir.join(format!("{name}.json"));
    ensure!(filename.exists(), "file {filename:?} not found");

    if confirm(format!("🚨 Confirm deleting file {filename:?}").bold())? {
        fs::remove_file(filename)?;
        println!("🗑️  Deleted!");
    }

    Ok(())
}

fn show(dir: &Path, name: &str) -> anyhow::Result<()> {
    let filename = dir.join(format!("{name}.json"));
    ensure!(filename.exists(), "file {filename:?} not found");

    let keystore_str = fs::read_to_string(filename)?;
    let keystore: Keystore = from_json_slice(keystore_str)?;

    print_json_pretty(keystore)
}

fn list(dir: &Path) -> anyhow::Result<()> {
    let mut keystores = vec![];
    for entry in dir.read_dir()? {
        let entry = entry?;
        let keystore_str = fs::read_to_string(entry.path())?;
        let keystore: Keystore = serde_json::from_str(&keystore_str)?;
        keystores.push(keystore);
    }

    print_json_pretty(keystores)
}
