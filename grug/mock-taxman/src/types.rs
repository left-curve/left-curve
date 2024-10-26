use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_math::Udec128,
    grug_types::Denom,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub fee_denom: Denom,
    pub fee_rate: Udec128,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InstantiateMsg {
    pub config: Config,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename = "snake_case")]
pub enum QueryMsg {
    Config {},
}
