use {
    dango_types::oracle::{PrecisionlessPrice, PriceSource},
    grug::{Denom, Map, Timestamp},
    pyth_types::{GuardianSet, GuardianSetIndex, PythId, PythLazerId},
};

pub const GUARDIAN_SETS: Map<GuardianSetIndex, GuardianSet> = Map::new("guardian_set");

pub const PRICE_SOURCES: Map<&Denom, PriceSource> = Map::new("price_source");

/// Map from PythId to (price, sequence). The sequence is used on update
/// to ensure that the price is more recent.
pub const PRICES: Map<PythId, (PrecisionlessPrice, u64)> = Map::new("price");

/// Map from PythLazerId to price.
pub const PYTH_LAZER_PRICES: Map<PythLazerId, PrecisionlessPrice> = Map::new("pyth_lazer_price");

/// Set of trusted signers for Pyth Lazer. The key is the public key of the
/// signer, and the value is the timestamp at which the signer is no longer
/// trusted.
pub const PYTH_LAZER_TRUSTED_SIGNERS: Map<&[u8], Timestamp> =
    Map::new("pyth_lazer_trusted_signers");
