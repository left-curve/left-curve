use {
    crate::auth::{AccountStatus, Nonce},
    std::collections::BTreeSet,
};

#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Whether this account is to be activated upon instantiation.
    /// If not, a minimum deposit is required to activate the account.
    pub activate: bool,
}

/// Query messages for the single-signature account
#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the account's status.
    #[returns(AccountStatus)]
    Status {},
    /// Query the most recent transaction nonces that have been recorded.
    #[returns(BTreeSet<Nonce>)]
    SeenNonces {},
}
