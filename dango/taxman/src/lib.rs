mod execute;
#[cfg(feature = "metrics")]
pub mod metrics;
mod query;
mod state;

pub use {execute::*, query::*, state::*};

/// Maximum age of volume data to store.
pub const MAX_VOLUME_AGE: grug::Duration = grug::Duration::from_weeks(3);
