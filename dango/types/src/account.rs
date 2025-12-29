/// Types relevant for multi-signature accounts.
pub mod multi;

/// Types relevant for single-signature accounts.
pub mod single;

/// Single- and multi-signature accounts share the same instantiate message.
#[grug::derive(Serde)]
pub struct InstantiateMsg {
    /// Whether this account is to be activated upon instantiation.
    /// If not, a minimum deposit is required to activate the account.
    pub activate: bool,
}
