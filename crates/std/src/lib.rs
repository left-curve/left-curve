// cw-std is a "meta crate", meaning it doesn't contain any content itself, but
// rather just re-export contents from other crates. The objective is that
// contract developers only needs to add one dependency that has everything they
// need.
pub use {cw_macros::*, cw_storage::*, cw_testing::*, cw_types::*, cw_wasm::*};

// dependencies used by the procedural macros
#[doc(hidden)]
pub mod __private {
    pub use {::borsh, ::serde, ::serde_with};
}
