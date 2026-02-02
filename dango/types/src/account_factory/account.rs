use crate::account_factory::UserIndex;

/// Global index of an account.
///
/// Used as salt to derive account addresses. This ensures the uniqueness of
/// account addresses.
pub type AccountIndex = u32;

/// Information of an account.
#[grug::derive(Serde, Borsh)]
pub struct Account {
    pub index: AccountIndex,

    /// User who owns the account.
    ///
    /// The user can sign transactions with any key associated with their
    /// username and this account as sender.
    pub owner: UserIndex,
}
