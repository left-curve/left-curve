use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{Binary, Expiration},
    serde::{Deserialize, Serialize},
};

/// Public key associated with an account.
///
/// Two cryptographic signature schemes are accepted: Secp256k1 and Secp256r1.
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum PublicKey {
    Secp256k1(Binary),
    Secp256r1(Binary),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstantiateMsg {
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum ExecuteMsg {
    /// Change the public key associated with the account to a new one.
    UpdateKey {
        new_public_key: PublicKey,
    },
    RemovedExpiredUnorderedTxs {
        limit: Option<u32>,
    },
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
    pub public_key: PublicKey,
    pub sequence: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct AccountData {
    pub order: TxOrder,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum TxOrder {
    Ordered,
    Unordered { expiration: Expiration },
}
