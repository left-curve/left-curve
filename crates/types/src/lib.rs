mod address;
mod api;
mod app;
mod bank;
mod binary;
mod coin;
mod context;
mod db;
mod decimal;
mod empty;
mod error;
mod event;
mod hash;
mod ibc;
mod macros;
mod math;
#[cfg(not(target_arch = "wasm32"))]
mod mocks;
mod query;
mod response;
mod result;
mod serde;
mod signed;
mod timestamp;
mod tx;
mod uint;
mod utils;

pub use {
    address::*, api::*, app::*, bank::*, binary::*, coin::*, context::*, db::*, decimal::*,
    empty::*, error::*, event::*, hash::*, ibc::*, math::*, query::*, response::*, result::*,
    serde::*, signed::*, timestamp::*, tx::*, uint::*, utils::*,
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
