use {
    crate::{Addr, Binary, Coins, Config, Hash},
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
    /// Update the chain-level configurations. Only the `owner` can do this.
    /// If `owner` is set to None, no one can update the config.
    SetConfig {
        new_cfg: Config,
    },
    /// Send coins to the given recipient address.
    ///
    /// Note that we don't assert the recipient is an account that exists, only
    /// that it's a valid 32-byte hex string. The sender is reponsible to make
    /// sure to put the correct address.
    Transfer {
        to: Addr,
        coins: Coins,
    },
    /// Upload a Wasm binary code and store it in the chain's state.
    Upload {
        wasm_byte_code: Binary,
    },
    /// Register a new account.
    Instantiate {
        code_hash: Hash,
        msg: Binary,
        salt: Binary,
        funds: Coins,
        admin: Option<Addr>,
    },
    /// Execute the contract.
    Execute {
        contract: Addr,
        msg: Binary,
        funds: Coins,
    },
    /// Update the `code_hash` associated with a contract.
    /// Only the contract's `admin` is authorized to do this. If the admin is
    /// set to None, no one can update the code hash.
    Migrate {
        contract: Addr,
        new_code_hash: Hash,
        msg: Binary,
    },
    /// Create a new IBC light client.
    CreateClient {
        code_hash: Hash,
        client_state: Binary,
        consensus_state: Binary,
        salt: Binary,
    },
    /// Update the state of an IBC light client by submitting a new header.
    UpdateClient {
        client_id: Addr,
        header: Binary,
    },
    /// Freeze an IBC light client by submitting evidence of a misbehavior.
    FreezeClient {
        client_id: Addr,
        misbehavior: Binary,
    },
}
