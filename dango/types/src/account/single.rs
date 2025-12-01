use crate::account_factory::UserIndex;

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
