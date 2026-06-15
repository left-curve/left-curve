use dango_primitives::{Duration, Timestamp};

// Re-use the same mock chain ID from Dango test suite.
pub use dango_primitives::MOCK_CHAIN_ID;

pub const MOCK_GENESIS_TIMESTAMP: Timestamp = Timestamp::from_days(365);

pub const MOCK_BLOCK_TIME: Duration = Duration::from_millis(250);

pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;
