mod execute;
mod query;
mod state;

pub use {execute::*, query::*, state::*};

/// The maximum number of accounts a user can be associated with.
pub const MAX_ACCOUNTS_PER_USER: usize = 5;
