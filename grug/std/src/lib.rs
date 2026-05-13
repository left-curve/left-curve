pub use {grug_macros::*, grug_math::*, grug_storage::*, grug_types::*};

// The FFI crate is only included when building for WebAssembly.
#[cfg(target_arch = "wasm32")]
pub use grug_ffi::*;

// The testing crate is only included when _not_ building for WebAssembly.
// It contains Wasm-incompatible features, such as async runtime, threads,
// and RNGs.
#[cfg(not(target_arch = "wasm32"))]
pub use grug_testing::*;
