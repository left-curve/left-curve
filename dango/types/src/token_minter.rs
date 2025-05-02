mod events;
mod msg;
mod types;

pub use {events::*, msg::*, types::*};

use {grug::Part, std::sync::LazyLock};

/// The namespace that synthetic tokens will be minted under. The bank contract
/// must give Warp contract admin power over this namespace.
///
/// Synthetic tokens will be given denoms with the format:
///
/// ```plain
/// bri/{chain_symbol}/{token_symbol}
/// ```
///
/// E.g.,
///
/// - `bri/btc/btc`
/// - `bri/xrp/xrp`
/// - `bri/sol/bonk`
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("bri"));

/// The subnamespace used for alloyed tokens.
///
/// E.g.,
///
/// - `bri/all/eth`
/// - `bri/all/usdc`
pub static ALLOY_SUBNAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("all"));
