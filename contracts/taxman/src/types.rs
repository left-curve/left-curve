use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::Udec128,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub fee_denom: String,
    pub fee_rate: Udec128,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ExecuteMsg {
    UpdateConfig { new_cfg: Config },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum QueryMsg {
    Config {},
}
