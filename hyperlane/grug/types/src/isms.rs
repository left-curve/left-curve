pub mod multisig;

use grug::HexBinary;

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(IsmQueryResponse)]
    Ism(IsmQuery),
}

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
    pub fn as_verify(self) {
        match self {
            IsmQueryResponse::Verify(res) => res,
        }
    }
}
