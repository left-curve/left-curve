use {
    grug_types::{Addr, Coins, Uint128},
    serde::{Deserialize, Serialize},
    std::collections::BTreeMap,
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InstantiateMsg {
    pub initial_balances: BTreeMap<Addr, Coins>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum ExecuteMsg {
    /// Mint a token of the specified amount to a user.
    Mint {
        to: Addr,
        denom: String,
        amount: Uint128,
    },
    /// Burn a token of the specified amount from a user.
    Burn {
        from: Addr,
        denom: String,
        amount: Uint128,
    },
    /// Forcibly transfer a token.
    /// This is used by the taxman to charge gas fee from a transaction's sender.
    ForceTransfer {
        from: Addr,
        to: Addr,
        denom: String,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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
