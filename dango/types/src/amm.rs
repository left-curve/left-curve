mod config;
mod msg;
mod namespace;
mod pool;

pub use {config::*, msg::*, namespace::*, pool::*};

use grug::Uint128;

/// The amount of liquidity shares that will be withheld by the AMM contract
/// during a pool's creation.
///
/// This is necessary for preventing an economic attack manipulating the
/// liquidity token's value. See:
/// <https://ethereum.stackexchange.com/questions/132491/why-minimum-liquidity-is-used-in-dex-like-uniswap>
pub const MINIMUM_LIQUIDITY: Uint128 = Uint128::new(1000);
