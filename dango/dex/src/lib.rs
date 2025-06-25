mod core;
mod cron;
mod execute;
mod query;
mod state;

pub use {core::*, cron::*, execute::*, query::*, state::*};

/// If an oracle price is older than this, it is not used for the logics in this contract.
pub const MAX_ORACLE_STALENESS: grug::Duration = grug::Duration::from_seconds(5);
