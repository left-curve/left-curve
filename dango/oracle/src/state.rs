use {
    dango_types::oracle::{PrecisionlessPrice, PriceSource},
    grug::{Denom, Map, Serde, Timestamp},
    pyth_types::PythLazerId,
};

pub const PRICE_SOURCES: Map<&Denom, PriceSource, Serde> = Map::new("price_source");

/// Map from PythLazerId to price.
pub const PYTH_LAZER_PRICES: Map<PythLazerId, PrecisionlessPrice> = Map::new("pyth_lazer_price");

/// Set of trusted signers for Pyth Lazer. The key is the public key of the
/// signer, and the value is the timestamp at which the signer is no longer
/// trusted.
pub const PYTH_LAZER_TRUSTED_SIGNERS: Map<&[u8], Timestamp> =
    Map::new("pyth_lazer_trusted_signers");
