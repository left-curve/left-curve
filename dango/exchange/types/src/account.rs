use {
    crate::auth::{AccountStatus, Nonce},
    dango_primitives::ByteArray,
    std::collections::BTreeSet,
};

#[dango_primitives::derive(Serde)]
pub struct InstantiateMsg {
    /// Whether this account is to be activated upon instantiation.
    /// If not, a minimum deposit is required to activate the account.
    pub activate: bool,
}

/// Query messages for the single-signature account
#[dango_primitives::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the account's status.
    #[returns(AccountStatus)]
    Status {},
    /// Query the most recent transaction nonces recorded for standard
    /// (master-key) credentials.
    #[returns(BTreeSet<Nonce>)]
    SeenNonces {},
    /// Query the most recent transaction nonces recorded for the given session
    /// key.
    #[returns(BTreeSet<Nonce>)]
    SessionSeenNonces { session_key: ByteArray<33> },
}
