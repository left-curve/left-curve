mod events;
mod msgs;
mod types;

pub use {events::*, msgs::*, types::*};

use {grug::Part, std::sync::LazyLock};

/// The namespace that synthetic tokens will be minted under. The bank contract
/// must give Warp contract admin power over this namespace.
///
/// Synthetic tokens will be given denoms with the format:
///
/// ```plain
/// hyp/{chain_symbol}/{token_symbol}
/// ```
///
/// E.g.,
///
/// - `hyp/btc/btc`
/// - `hyp/xrp/xrp`
/// - `hyp/sol/bonk`
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("hyp"));

/// The subnamespace used for alloyed tokens.
///
/// E.g.,
///
/// - `hyp/all/eth`
/// - `hyp/all/usdc`
pub static ALLOY_SUBNAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("all"));
