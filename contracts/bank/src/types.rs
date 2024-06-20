use {
    grug::{grug_derive, Addr, Coins, Uint128},
    std::collections::BTreeMap,
};

#[grug_derive(serde)]
pub struct InstantiateMsg {
    pub initial_balances: BTreeMap<Addr, Coins>,
}

#[grug_derive(serde)]
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
}

#[grug_derive(serde)]
pub enum QueryMsg {
    /// Enumerate all holders of a given token and their balances.
    /// Returns: `Vec<HoldersResponseItem>`.
    Holders {
        denom: String,
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}

#[grug_derive(serde)]
pub struct HoldersResponseItem {
    pub address: Addr,
    pub amount: Uint128,
}
