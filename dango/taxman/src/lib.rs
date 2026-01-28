mod execute;
mod query;
mod state;

pub use {execute::*, query::*, state::*};

/// Maximum referrer chain depth.
pub const MAX_REFERRER_CHAIN_DEPTH: u8 = 5;
