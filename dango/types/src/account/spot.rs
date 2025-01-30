use {crate::auth::Nonce, grug::Coins, std::collections::BTreeSet};

/// Query messages for the spot account
#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the most recent transaction nonces that have been recorded.
    #[returns(BTreeSet<Nonce>)]
    SeenNonces {},
}

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    pub at_least: Coins,
}
