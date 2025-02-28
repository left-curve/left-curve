mod events;
mod msgs;
mod types;

pub use {events::*, msgs::*, types::*};

use {grug::Part, std::sync::LazyLock};

/// The namespace used for dex.
///
/// E.g.,
///
/// - `dex/eth`
/// - `dex/usdc`
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("dex"));

/// The subnamespace used for lp tokens for the passive pools.
///
/// E.g.,
///
/// - `dex/lp/ethusdc`
/// - `dex/lp/btcusdc`
pub static LP_NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("lp"));
