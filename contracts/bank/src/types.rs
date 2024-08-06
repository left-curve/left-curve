use {
    borsh::{BorshDeserialize, BorshSerialize},
    grug_types::{Addr, Coins, Uint256},
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstantiateMsg {
    pub initial_balances: BTreeMap<Addr, Coins>,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum ExecuteMsg {
    /// Mint a token of the specified amount to a user.
    Mint {
        to: Addr,
        denom: String,
        amount: Uint256,
    },
    /// Burn a token of the specified amount from a user.
    Burn {
        from: Addr,
        denom: String,
        amount: Uint256,
    },
    /// Forcibly transfer a coin from an account to a receiver.
    /// Can only be called by the chain's taxman contract.
    /// Used by taxman to withhold pending transaction fees.
    ForceTransfer {
        from: Addr,
        to: Addr,
        denom: String,
        amount: Uint256,
    },
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum QueryMsg {
    /// Enumerate all holders of a given token and their balances.
    /// Returns: `BTreeMap<Addr, Uint128>`.
    Holders {
        denom: String,
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}
