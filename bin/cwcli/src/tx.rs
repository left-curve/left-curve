use {
    crate::{format::print_json_pretty, keyring::Keyring, query::do_abci_query},
    anyhow::anyhow,
    clap::Parser,
    cw_account::StateResponse,
    cw_std::{from_json, to_json, Addr, Binary, Coins, Config, Hash, Message, QueryRequest},
    serde::Serialize,
    std::{fs::File, io::Read, path::PathBuf},
    tendermint_rpc::{Client, HttpClient},
};

#[derive(Parser)]
pub enum TxCmd {
    /// Update the chain-level configurations
    UpdateConfig {
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
    Store {
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
    pub async fn run(
        self,
        rpc_addr: &str,
        key_dir: PathBuf,
        key_name: Option<String>,
        sender: Option<Addr>,
        chain_id: Option<String>,
        sequence: Option<u32>,
    ) -> anyhow::Result<()> {
        // compose the message
        let msg = match self {
            TxCmd::UpdateConfig { new_cfg } => {
                let new_cfg: Config = from_json(new_cfg.as_bytes())?;
                Message::UpdateConfig {
                    new_cfg,
                }
            },
            TxCmd::Transfer { to, coins } => {
                let coins: Coins = from_json(coins.as_bytes())?;
                Message::Transfer {
                    to,
                    coins,
                }
            },
            TxCmd::Store { path } => {
                let mut file = File::open(path)?;
                let mut wasm_byte_code = vec![];
                file.read_to_end(&mut wasm_byte_code)?;
                Message::StoreCode {
                    wasm_byte_code: wasm_byte_code.into(),
                }
            },
            TxCmd::Instantiate { code_hash, msg, salt, funds, admin } => {
                let funds = match funds {
                    Some(s) => from_json(s.as_bytes())?,
                    None => Coins::empty(),
                };
                Message::Instantiate {
                    code_hash,
                    msg: msg.into_bytes().into(),
                    salt: salt.into_bytes().into(),
                    funds,
                    admin,
                }
            },
            TxCmd::Execute { contract, msg, funds } => {
                let funds = match funds {
                    Some(s) => from_json(s.as_bytes())?,
                    None => Coins::empty(),
                };
                Message::Execute {
                    contract,
                    msg: msg.into_bytes().into(),
                    funds,
                }
            },
            TxCmd::Migrate { contract, new_code_hash, msg } => {
                Message::Migrate {
                    contract,
                    new_code_hash,
                    msg: msg.into_bytes().into(),
                }
            },
        };

        // create RPC client
        let client = HttpClient::new(rpc_addr)?;

        // query chain id
        let chain_id = match chain_id {
            None => {
                // TODO: avoid cloning here?
                do_abci_query(client.clone(), QueryRequest::Info {})
                .await?
                .as_info()
                .chain_id
            },
            Some(id) => id,
        };

        // query account sequence
        let sender = sender.ok_or(anyhow!("Sender address not provided"))?;
        let sequence = match sequence {
            None => {
                // TODO: avoid cloning here?
                from_json::<StateResponse>(do_abci_query(
                    client.clone(),
                    QueryRequest::WasmSmart {
                        contract: sender.clone(),
                        msg: to_json(&cw_account::QueryMsg::State {})?,
                    },
                )
                .await?
                .as_wasm_smart()
                .data)?
                .sequence
            },
            Some(seq) => seq,
        };

        // load signing key
        let key_name = key_name.ok_or(anyhow!("Key name not provided"))?;
        let keyring = Keyring::open(key_dir)?;
        let key = keyring.get(&key_name)?;

        // sign and broadcast the tx
        let tx = key.create_and_sign_tx(sender, vec![msg], &chain_id, sequence)?;
        let tx_bytes = to_json(&tx)?;
        let broadcast_res = client.broadcast_tx_async(tx_bytes).await?;

        print_json_pretty(PrintableBroadcastResponse {
            code: broadcast_res.code.into(),
            data: broadcast_res.data.to_vec().into(),
            log:  broadcast_res.log,
            hash: broadcast_res.hash.to_string(),
        })?;

        Ok(())
    }
}

#[derive(Serialize)]
struct PrintableBroadcastResponse {
    code: u32,
    data: Binary,
    log:  String,
    hash: String,
}
