use grug::Empty;

/// Types relevant for multi-signature accounts.
pub mod multi;

/// Types relevant for single-signature accounts.
pub mod single;

/// Types relevant for spot accounts.
pub mod spot;

/// Single- and multi-signature accounts share the same instantiate message,
/// which is just empty.
pub type InstantiateMsg = Empty;
