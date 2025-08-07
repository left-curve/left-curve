mod direction;
mod events;
mod msgs;
mod order;
mod pair;
mod price;

pub use {direction::*, events::*, msgs::*, order::*, pair::*, price::*};

use {grug::Part, std::sync::LazyLock};

/// The namespace used by the DEX contract.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("dex"));

/// The subnamespace used for LP tokens for the DEX passive pools.
///
/// The full denom is specified by the chain owner when creating a trading pair.
/// In general, it should be in the format:
///
/// ```plain
/// dex/pool/{base_denom}/{quote_denom}
/// ```
///
/// E.g.,
///
/// - `dex/pool/eth/usdc`
/// - `dex/pool/btc/usdc`
pub static LP_NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("pool"));
