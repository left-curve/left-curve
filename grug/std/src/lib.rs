pub use {grug_macros::*, grug_math::*, grug_storage::*, grug_types::*};

// The FFI crate is only included when building for WebAssembly.
#[cfg(target_arch = "wasm32")]
pub use grug_ffi::*;

// The client and testing crates are only included when _not_ building for
// WebAssembly. They contain Wasm-incompatible feature, such as async runtime,
// threads, and RNGs.
#[cfg(not(target_arch = "wasm32"))]
pub use {grug_client::*, grug_testing::*};
