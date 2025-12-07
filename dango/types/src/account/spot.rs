use {
    crate::auth::{AccountStatus, Nonce},
    grug::Empty,
    std::collections::BTreeSet,
};

pub type InstantiateMsg = Empty;

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
