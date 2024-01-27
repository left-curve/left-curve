use {clap::Parser, cw_keyring::Keyring, std::path::PathBuf};

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
    pub fn run(self, key_dir: PathBuf) -> anyhow::Result<()> {
        let keyring = Keyring::open(key_dir)?;
        match self {
            KeyCmd::Add {
                name,
                recover,
                coin_type,
            } => keyring.add(&name, recover, coin_type),
            KeyCmd::Delete {
                name,
            } => keyring.delete(&name),
            KeyCmd::Show {
                name,
            } => keyring.show(&name),
            KeyCmd::List => keyring.list(),
        }
    }
}
