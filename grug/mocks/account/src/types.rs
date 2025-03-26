use {
    grug_types::{Addr, ByteArray, JsonSerExt, Message, StdError, StdResult},
    serde::{Deserialize, Serialize},
    sha2::Sha256,
};

/// An Secp256k1 public key in compressed form.
pub type PublicKey = ByteArray<33>;

/// An Secp256k1 signature.
pub type Signature = ByteArray<64>;

pub struct SignDoc<'a> {
    pub sender: Addr,
    pub msgs: &'a [Message],
    pub chain_id: &'a str,
    pub sequence: u32,
}

// Generate the bytes that the sender of a transaction needs to sign.
//
// The bytes are defined as:
//
// ```plain
// bytes := hasher(json(msgs) | sender | chain_id | sequence)
// ```
//
// Parameters:
//
// - `hasher` is a hash function; this account implementation uses SHA2-256;
// - `msgs` is the list of messages in the transaction;
// - `sender` is a 32 bytes address of the sender;
// - `chain_id` is the chain ID in UTF-8 encoding;
// - `sequence` is the sender account's sequence in 32-bit big endian encoding.
//
// Chain ID and sequence are included in the sign bytes, as they are necessary
// for preventing replat attacks (e.g. user signs a transaction for chain A;
// attacker uses the signature to broadcast another transaction on chain B.)
impl grug_types::SignData for SignDoc<'_> {
    type Error = StdError;
    type Hasher = Sha256;

    fn to_prehash_sign_data(&self) -> StdResult<Vec<u8>> {
        let mut prehash = Vec::new();
        // That there are multiple valid ways that the messages can be serialized
        // into JSON. Here we use `grug::to_json_vec` as the source of truth.
        prehash.extend(self.msgs.to_json_vec()?);
        prehash.extend(self.sender.as_ref());
        prehash.extend(self.chain_id.as_bytes());
        prehash.extend(self.sequence.to_be_bytes());
        Ok(prehash)
    }
}

/// Schema for the account credentials expected in [`Tx::credential`](grug_types::Tx::credential).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub sequence: u32,
    pub signature: Signature,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstantiateMsg {
    /// The Secp256k1 public key to be associated with the account.
    pub public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum ExecuteMsg {
    /// Change the public key associated with the account to a new one.
    UpdateKey { new_public_key: PublicKey },
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
