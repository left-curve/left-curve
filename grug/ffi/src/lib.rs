mod exports;
mod imports;
mod memory;

// Note: We don't need to `pub use macros::*` here because the `#[macro_export]`
// annotation alrady does that. Rust macros work in quirky ways.
pub use {exports::*, imports::*, memory::*};

// Re-export the proc-macro.
#[cfg(feature = "macros")]
pub use grug_macros::export;
