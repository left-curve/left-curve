use {grug::Part, std::sync::LazyLock};

/// Namespace that tokens associated with the AMM will be minted under.
/// The AMM contract must be granted admin power over this namespace.
pub static NAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("amm"));

/// Sub-namespace that liquidity share tokens will be minted under.
pub static SUBNAMESPACE: LazyLock<Part> = LazyLock::new(|| Part::new_unchecked("pool"));
