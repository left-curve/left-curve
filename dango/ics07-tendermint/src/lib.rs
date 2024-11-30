//! # ICS07 Tendermint Light Client
//! This contract uses ibc-rs to implement the ICS07 Tendermint Light Client.

#![deny(clippy::nursery, clippy::pedantic, warnings, missing_docs)]

mod execute;
mod query;
mod state;

pub use execute::*;
pub use query::*;
