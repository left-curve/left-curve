use {
    crate::{
        account_factory::UserIndex,
        auth::{AccountStatus, Nonce},
    },
    std::collections::BTreeSet,
};

/// Parameters of a single-signature account.
#[grug::derive(Serde, Borsh)]
#[non_exhaustive]
pub struct Params {
    /// User who owns the account.
    ///
    /// The user can sign transactions with any key associated with their
    /// username and this account as sender.
    pub owner: UserIndex,
}

impl Params {
    pub fn new(owner: UserIndex) -> Self {
        Self { owner }
    }
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
