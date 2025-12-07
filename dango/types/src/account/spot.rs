use {
    crate::auth::{AccountStatus, Nonce},
    grug::Coins,
    std::collections::BTreeSet,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub minimum_deposit: Coins,
}

/// Query messages for the spot account
#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the account's status.
    #[returns(AccountStatus)]
    Status {},
    /// Query the most recent transaction nonces that have been recorded.
    #[returns(BTreeSet<Nonce>)]
    SeenNonces {},
}
