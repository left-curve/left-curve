use grug::Duration;

pub mod core;
mod cron;
mod execute;
mod liquidity_depth;
mod position_index;
mod price;
mod querier;
mod query;
mod state;
mod volume;

/// Lookback window for volume-tiered fee rate resolution.
pub const VOLUME_LOOKBACK: Duration = Duration::from_days(14);

pub use {
    cron::*, execute::*, liquidity_depth::*, position_index::*, price::*, querier::*, query::*,
    state::*, volume::*,
};
