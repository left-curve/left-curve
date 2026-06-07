use {
    crate::auth::{AccountStatus, Nonce},
    grug_types::ByteArray,
    std::collections::BTreeSet,
};

#[grug_types::derive(Serde)]
pub struct InstantiateMsg {
    /// Whether this account is to be activated upon instantiation.
    /// If not, a minimum deposit is required to activate the account.
    pub activate: bool,
}

/// Execute messages for the single-signature account.
///
/// All variants are restricted to the chain's owner.
#[grug_types::derive(Serde)]
pub enum ExecuteMsg {
    /// Freeze the account, preventing it from sending transactions or
    /// receiving transfers. Requires the account to currently be in the
    /// `Active` state.
    Freeze {},
    /// Unfreeze the account, restoring it to the `Active` state. Requires
    /// the account to currently be in the `Frozen` state.
    Unfreeze {},
}

/// Query messages for the single-signature account
#[grug_types::derive(Serde, QueryRequest)]
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
