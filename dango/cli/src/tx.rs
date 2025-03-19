use {
    crate::{
        config::Config,
        home_directory::HomeDirectory,
        prompt::{confirm, print_json_pretty, read_password},
    },
    clap::{Parser, Subcommand},
    colored::Colorize,
    config_parser::parse_config,
    dango_client::{SigningKey, SingleSigner},
    dango_types::config::AppConfig,
    grug_app::GAS_COSTS,
    grug_client::{GasOption, RpcSigningClient},
    grug_types::{json, Addr, Binary, Coins, Hash256, Json, JsonDeExt, Message, NonEmpty, Signer},
    std::{fs::File, io::Read, path::PathBuf, str::FromStr},
};

#[derive(Parser)]
pub struct TxCmd {
    /// Transaction sender's username
    #[arg(long)]
    username: String,

    /// Transaction sender's address
    #[arg(long)]
    address: Addr,

    /// Name of the key to sign transactions
    #[arg(long)]
    key: String,

    /// Account nonce [default: query from chain]
    #[arg(long)]
    nonce: Option<u32>,

    /// Amount of gas units to request [default: estimate]
    #[arg(long)]
    gas_limit: Option<u64>,

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
        /// Updates to the chain configuration
        #[arg(long)]
        new_cfg: Option<String>,
        /// Updates to the app configuration
        #[arg(long)]
        new_app_cfg: Option<String>,
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
        code_hash: Hash256,
        /// Instantiate message as a JSON string
        msg: String,
        /// Salt in UTF-8 encoding
        salt: String,
        /// Contract label
        #[arg(long)]
        label: Option<String>,
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
        new_code_hash: Hash256,
        /// Migrate message as a JSON string
        msg: String,
    },
}

impl TxCmd {
    pub async fn run(self, app_dir: HomeDirectory) -> anyhow::Result<()> {
        // Parse the config file.
        let cfg: Config = parse_config(app_dir.config_file())?;

        let msg = match self.subcmd {
            SubCmd::Configure {
                new_cfg,
                new_app_cfg,
            } => {
                let new_cfg = new_cfg.map(|s| s.deserialize_json()).transpose()?;
                let new_app_cfg = new_app_cfg
                    .map(|s| s.deserialize_json::<AppConfig>())
                    .transpose()?;
                Message::configure(new_cfg, new_app_cfg)?
            },
            SubCmd::Transfer { to, coins } => {
                let coins = Coins::from_str(&coins)?;
                Message::transfer(to, coins)?
            },
            SubCmd::Upload { path } => {
                let mut file = File::open(path)?;
                let mut code = vec![];
                file.read_to_end(&mut code)?;
                Message::upload(code)
            },
            SubCmd::Instantiate {
                code_hash,
                msg,
                salt,
                label,
                funds,
                admin,
            } => {
                let msg = msg.deserialize_json::<Json>()?;
                let funds = funds.map_or_else(|| Ok(Coins::new()), |s| Coins::from_str(&s))?;
                Message::instantiate(code_hash, &msg, salt, label, admin, funds)?
            },
            SubCmd::Execute {
                contract,
                msg,
                funds,
            } => {
                let msg = msg.deserialize_json::<Json>()?;
                let funds = funds.map_or_else(|| Ok(Coins::new()), |s| Coins::from_str(&s))?;
                Message::execute(contract, &msg, funds)?
            },
            SubCmd::Migrate {
                contract,
                new_code_hash,
                msg,
            } => {
                let msg = msg.deserialize_json::<Json>()?;
                Message::migrate(contract, new_code_hash, &msg)?
            },
        };

        let client = RpcSigningClient::connect(
            cfg.transactions.chain_id.clone(),
            cfg.tendermint.rpc_addr.as_str(),
        )?;

        let mut signer = {
            let key_path = app_dir.keys_dir().join(format!("{}.json", self.key));
            let password = read_password("🔑 Enter a password to encrypt the key".bold())?;
            let sk = SigningKey::from_file(&key_path, &password)?;
            let signer = SingleSigner::new(&self.username, self.address, sk)?;
            if let Some(nonce) = self.nonce {
                signer.with_nonce(nonce)
            } else {
                signer.query_nonce(&client).await?
            }
        };

        if self.simulate {
            let msgs = NonEmpty::new_unchecked(vec![msg]);
            let unsigned_tx = signer.unsigned_transaction(msgs, &cfg.transactions.chain_id)?;
            let outcome = client.simulate(&unsigned_tx).await?;
            print_json_pretty(outcome)?;
        } else {
            let gas_opt = if let Some(gas_limit) = self.gas_limit {
                GasOption::Predefined { gas_limit }
            } else {
                GasOption::Simulate {
                    scale: cfg.transactions.gas_adjustment,
                    // We always increase the simulated gas consumption by this
                    // amount, since signature verification is skipped during
                    // simulation.
                    flat_increase: GAS_COSTS.secp256k1_verify,
                }
            };

            let maybe_res = client
                .send_message_with_confirmation(&mut signer, msg, gas_opt, |tx| {
                    print_json_pretty(tx)?;
                    Ok(confirm("🤔 Broadcast transaction?".bold())?)
                })
                .await?;

            if let Some(res) = maybe_res {
                print_json_pretty(json!({
                    "code": res.code.value(),
                    "data": Binary::from(res.data.to_vec()),
                    "log":  res.log,
                    "hash": res.hash.to_string(),
                }))?;
            } else {
                println!("🤷 User aborted");
            }
        }

        Ok(())
    }
}
