use {
    crate::{Addr, Binary, Coins, Config, Hash, Json},
    serde::{Deserialize, Serialize},
    serde_with::skip_serializing_none,
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Tx {
    pub sender: Addr,
    pub msgs: Vec<Message>,
    pub credential: Binary,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    /// Update the chain-level configurations.
    ///
    /// Only the `owner` is authorized to do this. If the owner is set to `None`,
    /// no one can update the config.
    SetConfig { new_cfg: Config },
    /// Send coins to the given recipient address.
    Transfer { to: Addr, coins: Coins },
    /// Upload a Wasm binary code and store it in the chain's state.
    Upload { code: Binary },
    /// Register a new account.
    Instantiate {
        code_hash: Hash,
        msg: Json,
        salt: Binary,
        funds: Coins,
        admin: Option<Addr>,
    },
    /// Execute a contract.
    Execute {
        contract: Addr,
        msg: Json,
        funds: Coins,
    },
    /// Update the `code_hash` associated with a contract.
    ///
    /// Only the contract's `admin` is authorized to do this. If the admin is
    /// set to `None`, no one can update the code hash.
    Migrate {
        contract: Addr,
        new_code_hash: Hash,
        msg: Json,
    },
    /// Create a new IBC light client.
    CreateClient {
        code_hash: Hash,
        client_state: Json,
        consensus_state: Json,
        salt: Binary,
    },
    /// Update the state of an IBC light client by submitting a new header.
    UpdateClient { client_id: Addr, header: Json },
    /// Freeze an IBC light client by submitting evidence of a misbehavior.
    FreezeClient { client_id: Addr, misbehavior: Json },
}
