use {
    crate::auth::{Credential, Metadata},
    grug::Empty,
};

/// Types relevant for multi-signature accounts.
pub mod multi;

/// Types relevant for single-signature accounts.
pub mod single;

/// Single- and multi-signature accounts share the same instantiate message,
/// which is just empty.
pub type InstantiateMsg = Empty;

/// Transactions submitted to the Dango chain, with the Dango account metadata
/// and credential types.
pub type Tx = grug::Tx<Metadata, Credential>;
