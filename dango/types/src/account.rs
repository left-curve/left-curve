/// Types relevant for multi-signature accounts.
pub mod multi;

/// Types relevant for single-signature accounts.
pub mod single;

/// Single- and multi-signature accounts share the same instantiate message,
/// which is just empty.
pub type InstantiateMsg = grug::Empty;
