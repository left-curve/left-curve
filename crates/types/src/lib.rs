mod address;
mod app;
mod bank;
mod binary;
mod coin;
mod context;
mod db;
mod empty;
mod error;
mod event;
mod forward_ref;
mod hash;
mod ibc;
mod math;
#[cfg(not(target_arch = "wasm32"))]
mod mocks;
mod query;
mod response;
mod result;
mod serde;
mod timestamp;
mod traits;
mod tx;
mod utils;

pub use {
    address::*, app::*, bank::*, binary::*, coin::*, context::*, db::*, empty::*, error::*,
    event::*, hash::*, ibc::*, query::*, response::*, result::*, serde::*, timestamp::*, traits::*,
    tx::*, utils::*,
};

// Mocks need to be excluded in Wasm builds because they depend on k256/p256
// crates, which includes random operators.
#[cfg(not(target_arch = "wasm32"))]
pub use mocks::*;

/// Represents any valid JSON value, including numbers, booleans, strings,
/// objects, and arrays.
///
/// This is a re-export of `serde_json::Value`, but we rename it to "Json" to be
/// clearer what it is.
pub use serde_json::Value as Json;
