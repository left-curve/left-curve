// This is a "meta crate", meaning it doesn't contain any content itself, but
// rather just re-export contents from other crates. The objective is that
// contract developers only needs to add one single dependency that has
// everything they need.
pub use {grug_macros::*, grug_storage::*, grug_types::*};

// The FFI crate is only included when building for WebAssembly.
#[cfg(target_arch = "wasm32")]
pub use grug_ffi::*;

// The testing crate is only included when _not_ building for WebAseembly.
// It contains Wasm-incompatible operators, e.g. in `MockApi` which uses RNGs.
#[cfg(not(target_arch = "wasm32"))]
pub use grug_testing::*;

// Dependencies used by the procedural macros
#[doc(hidden)]
pub mod __private {
    pub use {::borsh, ::serde, ::serde_with};
}
