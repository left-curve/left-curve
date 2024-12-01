//! # ICS07 Tendermint Light Client
//! This contract uses ibc-rs to implement the ICS07 Tendermint Light Client.

#![deny(clippy::nursery, clippy::pedantic, warnings, missing_docs)]

mod execute;
pub mod ibc_rs_ctx;
mod query;

pub use execute::*;
pub use query::*;
