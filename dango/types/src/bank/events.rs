use {
    grug::{Addr, Denom, Uint128},
    std::collections::BTreeMap,
};

#[grug::derive(Serde)]
#[grug::event("balance_changes")]
pub struct BalanceChanges {
    pub address: Addr,
    pub changes: BTreeMap<Denom, (Uint128, BalanceChangeDirection)>,
}

#[grug::derive(Serde)]
pub enum BalanceChangeDirection {
    Increase,
    Decrease,
}
