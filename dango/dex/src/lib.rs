pub mod core;
mod cron;
mod execute;
pub mod liquidity_depth;
mod query;
mod state;

#[cfg(feature = "metrics")]
pub mod metrics;

pub use {cron::*, execute::*, query::*, state::*};

/// If an oracle price is older than this, it is not used for the logics in this contract.
pub const MAX_ORACLE_STALENESS: grug::Duration = grug::Duration::from_seconds(5);

/// The minimum amount of LP tokens that can exist for a pool with liquidity.
/// This are minted to and permanently locked in the DEX contract itself upon
/// the first liquidity provision.
///
/// This is necessary for preventing an type of attack that manipulates the
/// price of the LP token by donating funds to the DEX contract. See:
///
/// - https://github.com/Uniswap/v2-core/blob/master/contracts/UniswapV2Pair.sol#L119-L122
/// - https://ethereum.stackexchange.com/questions/132491/why-minimum-liquidity-is-used-in-dex-like-uniswap
///
/// We are less vulnerable to this than EVM-based protocols, because our
/// contract rejects unexpected transfers, but it is still a good idea to
/// implement this, just in case.
pub const MINIMUM_LIQUIDITY: grug::Uint128 = grug::Uint128::new(1_000);
