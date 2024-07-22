use {
    crate::prompt::{confirm, print_json_pretty, read_password},
    anyhow::anyhow,
    clap::{Parser, Subcommand},
    colored::Colorize,
    grug_sdk::{Client, SigningKey, SigningOptions},
    grug_types::{from_json_slice, Addr, Binary, Coins, Hash, Message, UnsignedTx},
    serde::Serialize,
    std::{fs::File, io::Read, path::PathBuf, str::FromStr},
    tendermint_rpc::endpoint::broadcast::tx_sync,
};

#[derive(Parser)]
pub struct TxCmd {
    /// Tendermint RPC address
    #[arg(long, default_value = "http://127.0.0.1:26657")]
    node: String,

    /// Name of the key to sign transactions
    #[arg(long)]
    key: Option<String>,

    /// Transaction sender address
    #[arg(long)]
    sender: Option<Addr>,

    /// Chain identifier [default: query from chain]
    #[arg(long)]
    chain_id: Option<String>,

    /// Account sequence number [default: query from chain]
    #[arg(long)]
    sequence: Option<u32>,

    /// Simulate gas usage without submitting the transaction to mempool.
    #[arg(long)]
    simulate: bool,

    #[command(subcommand)]
    subcmd: SubCmd,
}

#[derive(Subcommand)]
enum SubCmd {
    /// Update the chain-level configurations
    Configure {
        /// New configurations as a JSON string
        new_cfg: String,
    },
    /// Send coins to the given recipient address
    Transfer {
        /// Recipient address
        to: Addr,
        /// Coins to send in the format: {denom1}:{amount},{denom2}:{amount},...
        coins: String,
    },
    /// Update a Wasm binary code
    Upload {
        /// Path to the Wasm file
        path: PathBuf,
    },
    /// Instantiate a new contract
    Instantiate {
        /// Hash of the Wasm byte code to be associated with the contract
        code_hash: Hash,
        /// Instantiate message as a JSON string
        msg: String,
        /// Salt in UTF-8 encoding
        salt: String,
        /// Coins to be sent to the contract, in the format: {denom1}:{amount},{denom2}:{amount},...
        #[arg(long)]
        funds: Option<String>,
        /// Administrator address for the contract
        #[arg(long)]
        admin: Option<Addr>,
    },
    /// Execute a contract
    Execute {
        /// Contract address
        contract: Addr,
        /// Execute message as a JSON string
        msg: String,
        /// Coins to be sent to the contract, in the format: {denom1}:{amount},{denom2}:{amount},...
        #[arg(long)]
        funds: Option<String>,
    },
    /// Update the code hash associated with a contract
    Migrate {
        /// Contract address
        contract: Addr,
        /// New code hash
        new_code_hash: Hash,
        /// Migrate message as a JSON string
        msg: String,
    },
}

impl TxCmd {
    pub async fn run(self, key_dir: PathBuf) -> anyhow::Result<()> {
        let sender = self.sender.ok_or(anyhow!("sender not specified"))?;
        let key_name = self.key.ok_or(anyhow!("key name not specified"))?;

        // Compose the message
        let msg = match self.subcmd {
            SubCmd::Configure { new_cfg } => {
                let new_cfg = from_json_slice(new_cfg.as_bytes())?;
                Message::Configure { new_cfg }
            },
            SubCmd::Transfer { to, coins } => {
                let coins = Coins::from_str(&coins)?;
                Message::Transfer { to, coins }
            },
            SubCmd::Upload { path } => {
                let mut file = File::open(path)?;
                let mut code = vec![];
                file.read_to_end(&mut code)?;
                Message::Upload { code: code.into() }
            },
            SubCmd::Instantiate {
                code_hash,
                msg,
                salt,
                funds,
                admin,
            } => Message::Instantiate {
                msg: msg.into_bytes().into(),
                salt: salt.into_bytes().into(),
                funds: Coins::from_str(&funds.unwrap_or_default())?,
                code_hash,
                admin,
            },
            SubCmd::Execute {
                contract,
                msg,
                funds,
            } => Message::Execute {
                msg: msg.into_bytes().into(),
                funds: Coins::from_str(funds.as_deref().unwrap_or(Coins::EMPTY_COINS_STR))?,
                contract,
            },
            SubCmd::Migrate {
                contract,
                new_code_hash,
                msg,
            } => Message::Migrate {
                msg: msg.into_bytes().into(),
                new_code_hash,
                contract,
            },
        };

        let client = Client::connect(&self.node)?;

        if self.simulate {
            let unsigned_tx = UnsignedTx {
                sender,
                msgs: vec![msg],
            };
            let outcome = client.simulate(&unsigned_tx).await?;
            print_json_pretty(outcome)?;
        } else {
            // Load signing key
            let key_path = key_dir.join(format!("{key_name}.json"));
            let password = read_password("🔑 Enter a password to encrypt the key".bold())?;
            let signing_key = SigningKey::from_file(&key_path, &password)?;
            let sign_opts = SigningOptions {
                signing_key,
                sender,
                chain_id: self.chain_id,
                sequence: self.sequence,
            };

            // Broadcast transaction
            let maybe_res = client
                .send_message_with_confirmation(msg, &sign_opts, |tx| {
                    print_json_pretty(tx)?;
                    Ok(confirm("🤔 Broadcast transaction?".bold())?)
                })
                .await?;

            // Print result
            if let Some(res) = maybe_res {
                print_json_pretty(PrintableBroadcastResponse::from(res))?;
            } else {
                println!("🤷 User aborted");
            }
        }

        Ok(())
    }
}

/// Similar to tendermint_rpc Response but serializes to nicer JSON.
#[derive(Serialize)]
struct PrintableBroadcastResponse {
    code: u32,
    data: Binary,
    log: String,
    hash: String,
}

impl From<tx_sync::Response> for PrintableBroadcastResponse {
    fn from(broadcast_res: tx_sync::Response) -> Self {
        Self {
            code: broadcast_res.code.into(),
            data: broadcast_res.data.to_vec().into(),
            log: broadcast_res.log,
            hash: broadcast_res.hash.to_string(),
        }
    }
}
