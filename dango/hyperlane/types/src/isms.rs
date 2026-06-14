pub mod multisig;

use dango_primitives::HexBinary;

pub const HYPERLANE_DOMAIN_KEY: &str = "HYPERLANE";

// ----------------------------------- types -----------------------------------

#[dango_primitives::derive(Serde)]
pub enum IsmQuery {
    /// Verify a message.
    /// Return nothing is succeeds; throw error if fails.
    Verify {
        raw_message: HexBinary,
        raw_metadata: HexBinary,
    },
}

#[dango_primitives::derive(Serde)]
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

#[dango_primitives::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(IsmQueryResponse)]
    Ism(IsmQuery),
}
