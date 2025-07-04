mod core;
mod cron;
mod execute;
mod query;
mod state;

pub use {core::*, cron::*, execute::*, query::*, state::*};

/// If an oracle price is older than this, it is not used for the logics in this contract.
pub const MAX_ORACLE_STALENESS: grug::Duration = grug::Duration::from_seconds(5);

/// The minimum amount of LP tokens that can exist for a pool with liquidity. This are
/// minted to the Dex contract itself upon the first liquidity provision.
pub const MINIMUM_LP_TOKEN_AMOUNT: grug::Uint128 = grug::Uint128::new(1_000);
