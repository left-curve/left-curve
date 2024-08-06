use {
    grug_types::Binary,
    serde::{Deserialize, Serialize},
};

/// Schema for the account credentials expected in [`Tx::credential`](grug_types::Tx::credential).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub sequence: u32,
    pub signature: Binary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstantiateMsg {
    /// The Secp256k1 public key to be associated with the account.
    pub public_key: Binary,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum ExecuteMsg {
    /// Change the public key associated with the account to a new one.
    UpdateKey { new_public_key: Binary },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum QueryMsg {
    /// Query the state of the account, including its public key and sequence.
    /// Returns: [`StateResponse`]
    State {},
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct StateResponse {
    pub public_key: Binary,
    pub sequence: u32,
}
