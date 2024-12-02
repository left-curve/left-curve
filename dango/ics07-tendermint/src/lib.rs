//! # ICS07 Tendermint Light Client
//! This contract uses ibc-rs to implement the ICS07 Tendermint Light Client.

#![deny(clippy::nursery, clippy::pedantic, warnings, missing_docs)]

pub mod ctx;
mod execute;
mod query;

pub use {execute::*, query::*};
