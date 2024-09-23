//! This is a "meta crate", meaning it doesn't contain any content itself, but
//! rather just re-export contents from other crates.
//!
//! The objective is that contract developers only needs to add one single
//! dependency that has everything they need.

pub use {grug_macros::*, grug_math::*, grug_storage::*, grug_types::*};

// The FFI crate is only included when building for WebAssembly.
#[cfg(target_arch = "wasm32")]
pub use grug_ffi::*;

// The client and testing crates are only included when _not_ building for
// WebAssembly. They contain Wasm-incompatible feature, such as async runtime,
// threads, and RNGs.
#[cfg(not(target_arch = "wasm32"))]
pub use {grug_client::*, grug_testing::*};

// Dependencies used by the macros.
#[doc(hidden)]
pub mod __private {
    pub use {::borsh, ::serde, ::serde_with};
}
