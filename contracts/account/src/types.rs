use grug::{grug_derive, Binary};

/// Public key associated with an account.
///
/// Two cryptographic signature schemes are accepted: Secp256k1 and Secp256r1.
#[grug_derive(serde, borsh)]
pub enum PublicKey {
    Secp256k1(Binary),
    Secp256r1(Binary),
}

#[grug_derive(serde)]
pub struct InstantiateMsg {
    pub public_key: PublicKey,
}

#[grug_derive(serde)]
pub enum ExecuteMsg {
    // TODO: add a method to update the public key
}

#[grug_derive(serde)]
pub enum QueryMsg {
    /// Query the state of the account, including its public key and sequence.
    /// Returns: [`StateResponse`]
    State {},
}

#[grug_derive(serde)]
pub struct StateResponse {
    pub public_key: PublicKey,
    pub sequence: u32,
}
