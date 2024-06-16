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
    Mint {
        to: Addr,
        denom: String,
        amount: Uint128,
    },
    Burn {
        from: Addr,
        denom: String,
        amount: Uint128,
    },
}
