use crate::account_factory::{SignMode, Username};

/// Parameters of a single-signature account.
#[grug::derive(Serde, Borsh)]
pub struct Params {
    /// User who owns the account.
    ///
    /// The user can sign transactions with any key associated with their
    /// username and this account as sender.
    pub owner: Username,
    pub sign_mode: SignMode,
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the account's current sequence number.
    #[returns(u32)]
    Sequence {},
}
