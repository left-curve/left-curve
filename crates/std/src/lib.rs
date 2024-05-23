// Grug is a "meta crate", meaning it doesn't contain any content itself, but
// rather just re-export contents from other crates. The objective is that
// contract developers only needs to add one dependency that has everything they
// need.
pub use {grug_macros::*, grug_storage::*, grug_types::*, grug_wasm::*};

#[cfg(not(target_arch = "wasm32"))]
pub use grug_testing::*;

// dependencies used by the procedural macros
#[doc(hidden)]
pub mod __private {
    pub use {::borsh, ::serde, ::serde_with};
}
