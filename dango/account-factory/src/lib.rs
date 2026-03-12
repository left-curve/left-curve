mod account_querier;
mod execute;
mod query;
mod state;

pub use {account_querier::*, execute::*, query::*, state::*};

/// The maximum number of accounts a user can be associated with.
pub const MAX_ACCOUNTS_PER_USER: u8 = 5;
