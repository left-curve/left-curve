use {
    borsh::{BorshDeserialize, BorshSerialize},
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum IbcClientStatus {
    Active,
    Frozen,
    Expired,
}
