use crate::account_factory::Username;

/// Parameters of a single-signature account.
#[grug::derive(Serde, Borsh)]
pub struct Params {
    /// User who owns the account.
    ///
    /// The user can sign transactions with any key associated with their
    /// username and this account as sender.
    pub owner: Username,
}

#[grug::derive(Serde, QueryRequest)]
pub enum QueryMsg {
    /// Query the account's current sequence number.
    #[returns(u32)]
    Sequence {},
}
