use grug::Empty;

/// Types relevant for multi-signature accounts.
pub mod multi;

/// Types relevant for single-signature accounts.
pub mod single;

/// Types relevant for spot accounts.
pub mod spot;

/// Types relevant for margin accounts.
pub mod margin;

/// Single- and multi-signature accounts share the same instantiate message,
/// which is just empty.
pub type InstantiateMsg = Empty;

/// The status of an account. Only accounts in the `Active` state may send transactions.
#[grug::derive(Serde, Borsh)]
#[derive(Copy)]
pub enum AccountStatus {
    /// A freshly created account is in the "inactive" state. The user must make
    /// an initial deposit to activate it.
    Inactive,
    /// An account is activated once it receives a sufficient initial deposit.
    Active,
    /// an account may be frozen by the chain's owner. This feature does not exist yet.
    Frozen,
}

impl Default for AccountStatus {
    fn default() -> Self {
        AccountStatus::Inactive
    }
}
