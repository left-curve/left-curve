use {
    crate::Uint128,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Coin {
    pub denom: String,
    pub amount: Uint128,
}
