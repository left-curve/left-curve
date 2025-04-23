use {
    grug::{Addr, Coins, Denom, Udec256},
    std::collections::BTreeMap,
};

/// An event indicating a user has borrowed coins from the lending contract.
#[grug::derive(Serde)]
#[grug::event("borrowed")]
pub struct Borrowed {
    pub user: Addr,
    pub borrowed: Coins,
}

/// An event indicating a user has repaid coins to the lending contract.
#[grug::derive(Serde)]
#[grug::event("repaid")]
pub struct Repaid {
    pub user: Addr,
    pub repaid: Coins,
    pub refunds: Coins,
    pub remaining_scaled_debts: BTreeMap<Denom, Udec256>,
}
