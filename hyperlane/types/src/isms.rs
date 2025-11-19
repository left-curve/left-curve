pub mod multisig;

use grug::HexBinary;

pub const HYPERLANE_DOMAIN_KEY: &str = "HYPERLANE";

// ----------------------------------- types -----------------------------------

#[grug::derive(Serde)]
pub enum IsmQuery {
    /// Verify a message.
    /// Return nothing is succeeds; throw error if fails.
    Verify {
        raw_message: HexBinary,
        raw_metadata: HexBinary,
    },
}

#[grug::derive(Serde)]
pub enum IsmQueryResponse {
    Verify(()),
}

impl IsmQueryResponse {
    pub fn into_verify(self) {
        match self {
            IsmQueryResponse::Verify(res) => res,
        }
    }
}

// --------------------------------- messages ----------------------------------

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(IsmQueryResponse)]
    Ism(IsmQuery),
}
