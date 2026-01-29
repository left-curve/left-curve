mod execute;
#[cfg(feature = "metrics")]
pub mod metrics;
mod query;
mod state;

pub use {execute::*, query::*, state::*};

pub const VOLUME_TIME_GRANULARITY: grug::Duration = grug::Duration::from_days(1);

/// Maximum referrer chain depth.
pub const MAX_REFERRER_CHAIN_DEPTH: u8 = 5;
