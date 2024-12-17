use grug::{Binary, Json};

pub type Height = u64;

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    #[returns(VerifyCreationResponse)]
    VerifyCreation {
        client_state: Json,
        consensus_state: Json,
    },
}

#[grug::derive(Serde)]
pub struct VerifyCreationResponse {
    pub latest_height: Height,
    pub raw_client_state: Binary,
    pub raw_consensus_state: Binary,
}
